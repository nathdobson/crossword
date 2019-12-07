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
use std::collections::HashMap;
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
        solution: Option<u8>,
        answer: Option<u8>,
        across_clue: usize,
        down_clue: usize,
        rebus: Option<(u8, String)>,
        was_incorrect: bool,
        is_incorrect: bool,
        given: bool,
        circled: bool,
    },
}

#[derive(Clone, Debug)]
pub struct Clue {
    pub window: Window,
    pub clue: String,
}

#[derive(Debug, Clone)]
pub struct Puzzle {
    pub preamble: Vec<u8>,
    pub version: [u8; 4],
    pub reserved1: [u8; 2],
    pub reserved2: [u8; 12],
    pub title: String,
    pub author: String,
    pub copyright: String,
    pub grid: Grid<PuzzleCell>,
    pub clues: Vec<Clue>,
    pub note: String,
    pub has_rebus_index: bool,
    pub has_rebus_data: bool,
    pub has_style: bool,
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

impl Puzzle {
    fn from_raw(raw: RawPuzzle) -> Self {
        let mut grid =
            Grid::new(
                raw.solution.size(),
                |x, y|
                    match (raw.answer[(x, y)], raw.solution[(x, y)]) {
                        (b'.', b'.') => PuzzleCell::Black,
                        (answer, solution) => {
                            let style = raw.style.as_ref().map_or(0, |style| style[(x, y)]);
                            PuzzleCell::White {
                                across_clue: usize::max_value(),
                                down_clue: usize::max_value(),
                                answer: if answer == b'-' { None } else { Some(answer) },
                                solution: Some(solution),
                                rebus: match (&raw.rebus_index, &raw.rebus_data) {
                                    (Some(rebus_index), Some(rebus_data)) => {
                                        rebus_index[(x, y)].checked_sub(1).map(|rebus_index| {
                                            rebus_data.get(&rebus_index).map(|rebus| (rebus_index, rebus.clone())).unwrap()
                                        })
                                    }
                                    _ => None
                                },
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
            reserved1: raw.header.reserved1,
            reserved2: raw.header.reserved2,
            title: raw.title,
            author: raw.author,
            copyright: raw.copyright,
            grid: grid,
            clues: clues,
            note: raw.note,
            has_rebus_data: raw.rebus_data.is_some(),
            has_rebus_index: raw.rebus_index.is_some(),
            has_style: raw.style.is_some(),
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
        let mut rebus_data_map = BTreeMap::new();
        for y in 0..self.grid.size().1 {
            for x in 0..self.grid.size().0 {
                match &self.grid[(x, y)] {
                    PuzzleCell::White { rebus: Some((rebus_index, rebus)), .. } => {
                        rebus_data_map.insert(*rebus_index, rebus.clone());
                    }
                    _ => {}
                }
            }
        }
        let rebus_data = if self.has_rebus_data {
            Some(rebus_data_map)
        } else {
            None
        };
        let rebus_index = if self.has_rebus_index {
            Some(Grid::new(self.grid.size(), |x, y| {
                match &self.grid[(x, y)] {
                    PuzzleCell::White { rebus: Some((rebus_index, rebus)), .. } => 1 + *rebus_index,
                    _ => 0,
                }
            }))
        } else {
            None
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
        let style = if self.has_style {
            Some(style_grid)
        } else {
            None
        };
        RawPuzzle {
            header: RawHeader {
                preamble: self.preamble.clone(),
                version: self.version,
                reserved1: self.reserved1,
                reserved2: self.reserved2,
                width: self.grid.size().0 as u8,
                height: self.grid.size().1 as u8,
                clues: self.clues.len() as u16,
            },
            solution: Grid::new(self.grid.size(), |x, y| {
                match self.grid[(x, y)] {
                    PuzzleCell::Black => b'.',
                    PuzzleCell::White { solution, .. } => solution.unwrap_or(b'-'),
                }
            }),
            answer: Grid::new(self.grid.size(), |x, y| {
                match self.grid[(x, y)] {
                    PuzzleCell::Black => b'.',
                    PuzzleCell::White { answer, .. } => answer.unwrap_or(b'-'),
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

fn test_encode_decode() {
    let puzzle = Puzzle {
        preamble: vec![],
        version: *b"1.4\0",
        reserved1: [0u8; 2],
        reserved2: [0u8; 12],
        title: "".to_string(),
        author: "".to_string(),
        copyright: "".to_string(),
        grid: Grid::new((2, 2), |x, y| {}),
        clues: vec![],
        note: "".to_string(),
        has_rebus_index: false,
        has_rebus_data: false,
        has_style: false,
    };
}