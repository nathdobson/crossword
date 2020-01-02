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
use crate::core::puzzle::{Direction, Window, WindowMap};
use crate::play::raw_puzzle::MAGIC;

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Mode {
    Solving,
    Editing,
    EditingClue {
        cursor: usize,
    },
}

#[derive(Clone)]
pub struct View {
    pub position: (usize, usize),
    pub direction: Direction,
    pub mode: Mode,
    pub pencil: bool,
}

#[derive(Eq, Ord, PartialEq, PartialOrd, Clone, Debug)]
pub struct PuzzleCell {
    pub solution: String,
    pub answer: String,
    pub pencil: bool,
    pub was_incorrect: bool,
    pub is_incorrect: bool,
    pub given: bool,
    pub circled: bool,
}

impl Default for PuzzleCell {
    fn default() -> Self {
        PuzzleCell {
            solution: "".to_string(),
            answer: "".to_string(),
            pencil: false,
            was_incorrect: false,
            is_incorrect: false,
            given: false,
            circled: false,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Puzzle {
    pub preamble: Vec<u8>,
    pub version: [u8; 4],
    pub title: String,
    pub author: String,
    pub copyright: String,
    pub grid: Grid<Option<PuzzleCell>>,
    pub clues: WindowMap<String>,
    pub note: String,
}

/*pub fn windows<T>(grid: &Grid<T>, mut black: impl FnMut(&T) -> bool) -> Vec<Window> {
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
}*/


fn string_to_letter(string: &str) -> Option<u8> {
    if string.chars().count() == 1 {
        let c = string.chars().next().unwrap();
        if c >= 'A' && c <= 'Z' {
            return Some(c as u8);
        }
        if c >= 'a' && c <= 'z' {
            return Some(c.to_ascii_uppercase() as u8);
        }
    }
    None
}

impl Puzzle {
    fn from_raw(raw: RawPuzzle) -> Self {
        let grid =
            Grid::new(
                raw.solution.size(),
                |x, y|
                    match (raw.answer[(x, y)], raw.solution[(x, y)]) {
                        (b'.', b'.') => None,
                        (raw_answer, solution_letter) => {
                            let style = raw.style.as_ref().map_or(0, |style| style[(x, y)]);
                            let solution = match (&raw.rebus_index, &raw.rebus_data) {
                                (Some(rebus_index), Some(rebus_data)) => {
                                    rebus_index[(x, y)].checked_sub(1).map(|rebus_index| {
                                        rebus_data.get(&rebus_index).map(|rebus| (rebus.clone())).unwrap()
                                    })
                                }
                                _ => None
                            }.unwrap_or(if solution_letter == b'-' {
                                String::new()
                            } else {
                                String::from_utf8(vec![solution_letter]).unwrap()
                            });
                            let mut answer = "".to_string();
                            if let Some(rebus_user) = &raw.rebus_user {
                                answer = rebus_user[(x, y)].to_string();
                            }
                            if answer == "" && raw_answer != b'-' {
                                answer = String::from_utf8(vec![raw_answer]).unwrap();
                            }
                            Some(PuzzleCell {
                                solution,
                                answer,
                                was_incorrect: (style & 0x10) != 0,
                                is_incorrect: (style & 0x20) != 0,
                                given: (style & 0x40) != 0,
                                circled: (style & 0x80) != 0,
                                pencil: (style & 0x08) != 0,
                            })
                        }
                    });
        let mut windows: Vec<Window> = WindowMap::from_grid(&Grid::new(grid.size(), |x, y| {
            grid[(x, y)].is_some()
        })).windows().collect();
        windows.sort_by_key(|win| { (win.position().1, win.position().0, win.direction()) });
        assert_eq!(windows.len(), raw.clues.len());
        let clues =
            WindowMap::new(
                windows.iter()
                    .zip(raw.clues.iter())
                    .map(|(&window, clue)| { (window, clue.clone()) }),
                grid.size());
        let result = Puzzle {
            preamble: raw.header.preamble,
            version: raw.header.version,
            title: raw.title,
            author: raw.author,
            copyright: raw.copyright,
            grid: grid,
            clues,
            note: raw.note,
        };
        return result;
    }

    pub fn read_from(read: &mut dyn BufRead) -> io::Result<Puzzle> {
        Ok(Self::from_raw(RawPuzzle::read_from(read)?))
    }

    pub fn into_raw(&self) -> RawPuzzle {
        let mut clues =
            self.clues.iter()
                .map(|(window, clue)| (window, clue.clone()))
                .collect::<Vec<_>>();
        clues.sort_by_key(|(window, clue)| (window.position().1, window.position().0, window.direction()));
        let rebuses: BTreeSet<String> = self.grid.iter().filter_map(|cell| {
            match cell {
                Some(PuzzleCell { solution, .. }) => {
                    if solution == "" || string_to_letter(solution).is_some() {
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
                     Some(PuzzleCell { solution, .. }) =>
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
                None => 0,
                Some(PuzzleCell {
                         pencil,
                         was_incorrect,
                         is_incorrect,
                         given,
                         circled,
                         ..
                     }) => {
                    let mut bitmap = 0;
                    if pencil { bitmap |= 0x08 }
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
        let rebus_user_grid: Grid<String> = Grid::new(self.grid.size(), |x, y| {
            match &self.grid[(x, y)] {
                None => "".to_string(),
                Some(PuzzleCell { answer, .. }) => {
                    if string_to_letter(answer).is_none() {
                        answer.to_string()
                    } else {
                        "".to_string()
                    }
                }
            }
        });
        let rebus_user = if rebus_user_grid.iter().any(|x| !x.is_empty()) {
            Some(rebus_user_grid)
        } else {
            None
        };
        let result = RawPuzzle {
            header: RawHeader {
                preamble: self.preamble.clone(),
                version: self.version,
                reserved1: [0u8; 2],
                reserved2: [0u8; 12],
                width: self.grid.size().0 as u8,
                height: self.grid.size().1 as u8,
                clues: clues.len() as u16,
            },
            solution: Grid::new(self.grid.size(), |x, y| {
                match &self.grid[(x, y)] {
                    None => b'.',
                    Some(PuzzleCell { solution, .. }) =>
                        string_to_letter(solution).unwrap_or(b'-')
                }
            }),
            answer: Grid::new(self.grid.size(), |x, y| {
                match &self.grid[(x, y)] {
                    None => b'.',
                    Some(PuzzleCell { answer, .. }) =>
                        string_to_letter(answer).unwrap_or(b'-'),
                }
            }),
            rebus_index: rebus_index,
            rebus_data: rebus_data,
            rebus_user: rebus_user,
            play_data: None,
            style: style,
            title: self.title.clone(),
            author: self.author.clone(),
            copyright: self.copyright.clone(),
            clues: clues.into_iter().map(|(window, clue)| clue).collect(),
            note: self.note.clone(),
        };
        result
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
                None
            } else {
                Some(PuzzleCell {
                    solution: match (x, y) {
                        (0, 0) => "REB".to_string(),
                        (2, 2) => "BER".to_string(),
                        _ => "X".to_string()
                    },
                    answer: match (x, y) {
                        (0, 0) => "USR".to_string(),
                        (2, 2) => "RSU".to_string(),
                        _ => "Y".to_string()
                    },
                    pencil: false,
                    was_incorrect: false,
                    is_incorrect: false,
                    given: false,
                    circled: x == 2 && y == 0,
                })
            }
        }),
        clues: WindowMap::new([
                                  Window::new((0, 0), 3, Direction::Across),
                                  Window::new((0, 2), 3, Direction::Across),
                                  Window::new((0, 0), 3, Direction::Down),
                                  Window::new((2, 0), 3, Direction::Down),
                              ].iter().enumerate().map(|(index, &window)| {
            (window, format!("clue {}", index))
        }), (3, 3)),
        note: "Note".to_string(),
    };
    let mut data = vec![];
    puzzle.clone().write_to(&mut &mut data).unwrap();
    let puzzle2 = Puzzle::read_from(&mut data.as_slice()).unwrap();
    assert_eq!(puzzle, puzzle2);
}