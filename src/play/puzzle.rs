use std::path::Path;
use std::io::BufRead;
use std::io::BufReader;
use std::fs::File;
use std::io::Read;
use std::ops::Deref;
use std::convert::AsMut;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fmt::Debug;
use std::fmt;
use std::fmt::Formatter;
use std::io;
use super::raw_puzzle::RawPuzzle;
use std::io::Write;
use std::fs;
use std::ops::Range;
use super::range_split::RangeSplitExt;
use super::raw_puzzle::RawHeader;
use std::ffi::OsStr;
use std::collections::{HashMap, BTreeSet};
use std::collections::BTreeMap;
use crate::util::grid::Grid;
use crate::core::puzzle::{Direction, Window};
use crate::play::raw_puzzle::MAGIC;

#[derive(Clone)]
pub struct View {
    pub position: (usize, usize),
    pub direction: Direction,
}

#[derive(Eq, Ord, PartialEq, PartialOrd, Clone, Debug)]
pub enum PuzzleCell {
    Black,
    White {
        solution: String,
        answer: Option<String>,
        across_clue: usize,
        down_clue: usize,
        was_incorrect: bool,
        is_incorrect: bool,
        given: bool,
        circled: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Clue {
    pub window: Window,
    pub clue: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Puzzle {
    pub preamble: Vec<u8>,
    pub version: [u8; 4],
    pub title: String,
    pub author: String,
    pub copyright: String,
    pub grid: Grid<PuzzleCell>,
    pub clues: Vec<Clue>,
    pub note: String,
}

pub fn windows<T>(grid: &Grid<T>, mut black: impl FnMut(&T) -> bool) -> Vec<Window> {
    let mut result = vec![];
    for y in 0..grid.size().1 {
        let ranges = (0..grid.size().0).range_split(|&x| {
            black(&grid[(x, y)])
        }).collect::<Vec<Range<usize>>>();
        for range in ranges {
            if range.end - range.start > 1 {
                result.push(Window::new(
                    (range.start, y),
                    range.end - range.start,
                    Direction::Across,
                ));
            }
        }
    }
    for x in 0..grid.size().0 {
        let ranges = (0..grid.size().1).range_split(|&y| {
            black(&grid[(x, y)])
        }).collect::<Vec<Range<usize>>>();
        for range in ranges {
            if range.end - range.start > 1 {
                result.push(Window::new(
                    (x, range.start),
                    range.end - range.start,
                    Direction::Down,
                ));
            }
        }
    }

    result
}

fn string_to_letter(string: &str) -> Option<u8> {
    if string.len() == 1 {
        let c = string.chars().next().unwrap();
        if c >= 'A' && c <= 'Z' {
            return Some(c as u8);
        }
    }
    None
}

impl Puzzle {
    fn from_raw(raw: RawPuzzle) -> Self {
        let mut grid =
            Grid::new(
                raw.solution.size(),
                |x, y|
                    match (raw.answer[(x, y)], raw.solution[(x, y)]) {
                        (b'.', b'.') => PuzzleCell::Black,
                        (answer, solution_letter) => {
                            let style = raw.style.as_ref().map_or(0, |style| style[(x, y)]);
                            let solution = match (&raw.rebus_index, &raw.rebus_data) {
                                (Some(rebus_index), Some(rebus_data)) => {
                                    rebus_index[(x, y)].checked_sub(1).map(|rebus_index| {
                                        rebus_data.get(&rebus_index).map(|rebus| (rebus.clone())).unwrap()
                                    })
                                }
                                _ => None
                            }.unwrap_or(String::from_utf8(vec![solution_letter]).unwrap());
                            PuzzleCell::White {
                                across_clue: usize::max_value(),
                                down_clue: usize::max_value(),
                                answer: if answer == b'-' { None } else { Some(String::from_utf8(vec![answer]).unwrap()) },
                                solution,
                                was_incorrect: (style & 0x10) != 0,
                                is_incorrect: (style & 0x20) != 0,
                                given: (style & 0x40) != 0,
                                circled: (style & 0x80) != 0,
                            }
                        }
                    });
        let mut wins: Vec<Window> = windows(&grid, |cell| {
            match cell {
                PuzzleCell::Black => true,
                _ => false
            }
        });
        wins.sort_by_key(|win| { (win.position().1, win.position().0, win.direction()) });
        assert_eq!(wins.len(), raw.clues.len());
        let mut clues: Vec<Clue> = wins.iter().zip(raw.clues.iter()).map(|(&window, clue)| { Clue { window, clue: clue.clone() } }).collect();
        clues.sort_by_key(|clue| (clue.window.direction(), clue.window.position().1, clue.window.position().0));
        for (clue_index, clue) in clues.iter_mut().enumerate() {
            match clue.window.direction() {
                Direction::Across => {
                    for x in clue.window.position().0..clue.window.position().0 + clue.window.length() {
                        match &mut grid[(x, clue.window.position().1)] {
                            PuzzleCell::White { across_clue, .. } => {
                                assert_eq!(*across_clue, usize::max_value());
                                *across_clue = clue_index
                            }
                            _ => {}
                        }
                    }
                }
                Direction::Down => {
                    for y in clue.window.position().1..clue.window.position().1 + clue.window.length() {
                        match &mut grid[(clue.window.position().0, y)] {
                            PuzzleCell::White { down_clue, .. } => {
                                assert_eq!(*down_clue, usize::max_value());
                                *down_clue = clue_index
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        let result = Puzzle {
            preamble: raw.header.preamble,
            version: raw.header.version,
            title: raw.title,
            author: raw.author,
            copyright: raw.copyright,
            grid: grid,
            clues: clues,
            note: raw.note,
        };
        for (clue_index, clue) in result.clues.iter().enumerate() {
            assert_eq!(Some(clue_index), result.get_clue(&View { position: clue.window.position(), direction: clue.window.direction() }));
        }
        return result;
    }
    pub fn get_clue(&self, view: &View) -> Option<usize> {
        match self.grid[view.position] {
            PuzzleCell::White { across_clue, down_clue, .. } => {
                match view.direction {
                    Direction::Across => Some(across_clue),
                    Direction::Down => Some(down_clue)
                }
            }
            PuzzleCell::Black => None
        }
    }

    pub fn read_from(read: &mut dyn BufRead) -> io::Result<Puzzle> {
        Ok(Self::from_raw(RawPuzzle::read_from(read)?))
    }

    pub fn into_raw(mut self) -> RawPuzzle {
        self.clues.sort_by_key(|clue| (clue.window.position().1, clue.window.position().0, clue.window.direction()));
        let rebuses: BTreeSet<String> = self.grid.iter().filter_map(|cell| {
            match cell {
                PuzzleCell::White { solution, .. } => {
                    if string_to_letter(solution).is_some() {
                        None
                    } else {
                        Some(solution.clone())
                    }
                }
                _ => None
            }
        }).collect();
        let (rebus_data, rebus_index) = if rebuses.len() > 0 {
            (Some(rebuses.iter().cloned().enumerate().map(|(index, rebus)| (index as u8, rebus)).collect()),
             Some(Grid::new(self.grid.size(), |x, y| {
                 match &self.grid[(x, y)] {
                     PuzzleCell::White { solution, .. } =>
                         rebuses.iter()
                             .position(|rebus| rebus == solution)
                             .map(|x| x + 1).unwrap_or(0) as u8,
                     _ => 0u8,
                 }
             }))
            )
        } else {
            (None, None)
        };
        let style_grid = Grid::new(self.grid.size(), |x, y| {
            match self.grid[(x, y)] {
                PuzzleCell::Black => 0,
                PuzzleCell::White {
                    was_incorrect,
                    is_incorrect,
                    given,
                    circled, ..
                } => {
                    let mut bitmap = 0;
                    if was_incorrect { bitmap |= 0x10; }
                    if is_incorrect { bitmap |= 0x20; }
                    if given { bitmap |= 0x40; }
                    if circled { bitmap |= 0x80; }
                    bitmap
                }
            }
        });
        let style = if style_grid.iter().any(|&x| x != 0) {
            Some(style_grid)
        } else {
            None
        };
        RawPuzzle {
            header: RawHeader {
                preamble: self.preamble.clone(),
                version: self.version,
                reserved1: [0u8; 2],
                reserved2: [0u8; 12],
                width: self.grid.size().0 as u8,
                height: self.grid.size().1 as u8,
                clues: self.clues.len() as u16,
            },
            solution: Grid::new(self.grid.size(), |x, y| {
                match &self.grid[(x, y)] {
                    PuzzleCell::Black => b'.',
                    PuzzleCell::White { solution, .. } =>
                        string_to_letter(solution).unwrap_or(b'-')
                }
            }),
            answer: Grid::new(self.grid.size(), |x, y| {
                match &self.grid[(x, y)] {
                    PuzzleCell::Black => b'.',
                    PuzzleCell::White { answer, .. } =>
                        answer.as_ref().and_then(|string| string_to_letter(&string)).unwrap_or(b'-'),
                }
            }),
            rebus_index: rebus_index,
            rebus_data: rebus_data,
            style: style,
            title: self.title,
            author: self.author,
            copyright: self.copyright,
            clues: self.clues.into_iter().map(|clue| clue.clue).collect::<Vec<_>>(),
            note: self.note,
        }
    }

    pub fn write_to(self, write: &mut dyn Write) -> io::Result<()> {
        self.into_raw().write_to(write)
    }
}

//#[test]
//fn test_round_trip() {
//    for filename_result in fs::read_dir("puzzles").unwrap() {
//        let mut filename = filename_result.unwrap().path();
//        if filename.extension() == Some(OsStr::new("puz")) {
//            println!("Reading {:?}", filename);
//            let data = fs::read(&filename).unwrap();
//            let mut data_reader: &[u8] = &data;
//            let puzzle = Puzzle::read_from(&mut data_reader).unwrap();
//            let mut new_data: Vec<u8> = vec![];
//            puzzle.write_to(&mut new_data).unwrap();
//            filename.set_extension("puz.test");
//            fs::write(&filename, &new_data).unwrap();
//            assert!(data == new_data);
//        }
//    }
//}

#[test]
fn test_encode_decode() {
    let puzzle = Puzzle {
        preamble: vec![],
        version: *b"1.4\0",
        title: "Title".to_string(),
        author: "Author".to_string(),
        copyright: "Copyright".to_string(),
        grid: Grid::new((3, 3), |x, y| {
            if x == 1 && y == 1 {
                PuzzleCell::Black
            } else {
                PuzzleCell::White {
                    solution: match (x, y) {
                        (0, 0) => "REB".to_string(),
                        (2, 2) => "BER".to_string(),
                        _ => "x".to_string()
                    },
                    answer: None,
                    across_clue: match y {
                        0 => 0,
                        1 => usize::max_value(),
                        2 => 1,
                        _ => panic!()
                    },
                    down_clue: match x {
                        0 => 2,
                        1 => usize::max_value(),
                        2 => 3,
                        _ => panic!(),
                    },
                    was_incorrect: false,
                    is_incorrect: false,
                    given: false,
                    circled: x == 2 && y == 0,
                }
            }
        }),
        clues: [
            Window::new((0, 0), 3, Direction::Across),
            Window::new((0, 2), 3, Direction::Across),
            Window::new((0, 0), 3, Direction::Down),
            Window::new((2, 0), 3, Direction::Down),
        ].iter().enumerate().map(|(index, &window)| {
            Clue {
                window,
                clue: format!("clue {}", index),
            }
        }).collect(),
        note: "Note".to_string(),
    };
    let mut data = vec![];
    puzzle.clone().write_to(&mut &mut data).unwrap();
    let puzzle2 = Puzzle::read_from(&mut data.as_slice()).unwrap();
    assert_eq!(puzzle, puzzle2);
}