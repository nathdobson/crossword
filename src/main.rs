#![allow(dead_code, non_snake_case, unused_variables, unused_imports)]
#![feature(box_syntax, step_trait, rustc_private, const_fn)]

#[macro_use]
extern crate itertools;
#[macro_use]
extern crate lazy_static;
extern crate getopts;

use crate::fill::scored_list::ScoredWord;
use crate::fill::trie::Trie;
use crate::core::word::Word;
use crate::core::letter::Letter;
use crate::util::grid::Grid;
use std::{io, fmt, fs, iter};
use crate::play::puzzle::{Puzzle, PuzzleCell, View, Mode};
use std::fmt::{Display, Formatter};
use csv::ReaderBuilder;
use crate::core::puzzle::{Window, WindowMap, AsciiGrid, Direction, Cell};
use crate::fill::search::{Search, Canceled, take_one_result};
use std::collections::{HashSet, HashMap};
use std::io::{BufRead, stdout, stdin, Write};
use crate::play::interface::{TerminalOutput, start_rendering, stop_rendering, TerminalInput, RawScope};
use crate::play::play::Play;
use crate::play::puzzle::Mode::Editing;
use crate::fill::dictionary::EditedDictionary;
use getopts::Options;
use std::env;
use std::num::ParseIntError;

pub mod util;
pub mod core;
pub mod fill;
pub mod play;

fn create(filename: &str, width: usize, height: usize) -> io::Result<()> {
    let puzzle = Puzzle {
        preamble: vec![],
        version: *b"1.4\0",
        title: "Title".to_string(),
        author: "Author".to_string(),
        copyright: "Copyright".to_string(),
        grid: Grid::new((width, height), |x, y| { Some(PuzzleCell::default()) }),
        clues: WindowMap::new(WindowMap::from_grid(
            &Grid::new((width, height), |x, y| true))
                                  .windows().zip(iter::repeat("".to_string())), (width, height)),
        note: "".to_string(),
    };
    let mut data = vec![];
    puzzle.write_to(&mut &mut data)?;
    fs::write(filename, data)?;
    Ok(())
}

fn interface(filename: &str, edit: bool) -> io::Result<()> {
    let raw = RawScope::new();
    let data = fs::read(filename)?;
    let mut puzzle = Puzzle::read_from(&mut data.as_slice())?;
    let mut dictionary =
        EditedDictionary::new(
            ScoredWord::default().unwrap().iter().map(|sw| sw.word).collect(),
            "dictionaries/updates.csv");
    let mut view = View {
        position: (0, 0),
        direction: Direction::Across,
        mode: if edit { Mode::Editing } else { Mode::Solving },
        pencil: false,
    };
    let mut stdout = stdout();
    let mut stdin = stdin();
    start_rendering(&mut stdout)?;
    let mut input = TerminalInput { input: &mut stdin };
    loop {
        let mut output = vec![];
        TerminalOutput {
            output: &mut &mut output,
            view: &view,
            puzzle: &puzzle,
        }.render()?;
        stdout.write_all(&output)?;
        if let Some(next) = input.read_event()? {
            let mut play = Play::new(&mut view, &mut puzzle, Some(&mut dictionary));
            play.do_action(next);
        } else { break; }
    }
    stop_rendering(&mut stdout)?;
    let mut data = vec![];
    puzzle.write_to(&mut &mut data)?;
    fs::write(filename, data)?;
    Ok(())
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [create <FILE> <WIDTH> <HEIGHT>|edit <FILE>|play <FILE>] [options]", program);
    print!("{}", opts.usage(&brief));
}


#[derive(Debug)]
struct ParseError(String);

impl<E: Display> From<E> for ParseError {
    fn from(x: E) -> Self {
        ParseError(format!("{}", x))
    }
}

fn main_impl(args: &[String], matches: getopts::Matches) -> Result<(), ParseError> {
    if matches.opt_present("h") {
        return Err(ParseError(format!("")));
    }
    if matches.free.is_empty() {
        return Err(ParseError(format!("No arguments")));
    }
    match matches.free[0].as_str() {
        "create" => {
            if matches.free.len() != 4 {
                return Err(ParseError(format!("Need 3 arguments")));
            }
            let file = &matches.free[1];
            let width = matches.free[2].parse::<usize>()?;
            let height = matches.free[3].parse::<usize>()?;
            create(file, width, height)?;
        }
        "edit" => {
            if matches.free.len() != 2 {
                return Err(ParseError(format!("Need 1 argument")));
            }
            let file = &matches.free[1];
            interface(file, true)?;
        }
        "play" => {
            if matches.free.len() != 2 {
                return Err(ParseError(format!("Need 1 arguments")));
            }
            let file = &matches.free[1];
            interface(file, false)?;
        }
        _ => {
            return Err(ParseError(format!("Unknown command")));
        }
    }
    return Ok(());
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(e) => {
            eprintln!("{}", e);
            print_usage(&args[0], opts);
            return;
        }
    };
    match main_impl(&args, matches) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("{:?}", e);
            print_usage(&args[0], opts);
            return;
        }
    }
}