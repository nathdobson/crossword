#![allow(dead_code, non_snake_case, unused_variables, unused_imports)]
#![feature(box_syntax, step_trait, wait_until, rustc_private, const_fn)]

#[macro_use]
extern crate itertools;
#[macro_use]
extern crate lazy_static;

use crate::fill::scored_list::ScoredWord;
use crate::fill::trie::Trie;
use crate::core::word::Word;
use crate::core::letter::Letter;
use crate::util::grid::Grid;
use std::{io, fmt, fs};
use crate::play::puzzle::{Puzzle, PuzzleCell, View};
use std::fmt::{Display, Formatter};
use csv::ReaderBuilder;
use crate::core::puzzle::{Window, WindowMap, Cell, AsciiGrid, Direction};
use crate::fill::search::{Search, Canceled, take_one_result};
use std::collections::{HashSet, HashMap};
use std::io::{BufRead, stdout, stdin, Write};
use crate::play::interface::{TerminalOutput, start_rendering, stop_rendering, TerminalInput, RawScope};
use crate::play::play::Play;

pub mod util;
pub mod core;
pub mod fill;
pub mod play;

/*fn search(width: usize, height: usize, words: &[Word], tries: &[&Trie], acrosses: &mut Vec<Word>) {
    if height == acrosses.len() {
        let downs: Vec<Word> = (0..width).map(|x| (0..height).map(|y| acrosses[y][x]).collect()).collect();
        if !iproduct!(acrosses.iter(), downs.iter()).any(|(a, b)| a == b) {
            println!("{:?} {:?}", acrosses, downs);
        }
        return;
    }
    let mut tries2: Vec<&Trie> = tries.iter().cloned().collect();
    for &word in words {
        let mut min_size = usize::max_value();
        for x in 0..width {
            tries2[x] = tries[x].child(word[x]);
            min_size = min_size.min(tries2[x].len());
        }
        if min_size > 0 {
            acrosses.push(word);
            search(width, height, words, &tries2, acrosses);
            acrosses.pop();
        }
    }
}*/

/*fn main() {
    let dictionary = ScoredWord::default().unwrap();
    let width = 3;
    let height = 9;
    let mut acrosses: Vec<Word> =
        dictionary.iter()
            .filter_map(|w| if w.word.len() == width { Some(w.word) } else { None })
            .collect();
    acrosses.sort();
    let downs: Trie =
        dictionary.iter()
            .filter_map(|w| if w.word.len() == height { Some(w.word) } else { None })
            .collect();
    search(width, height, &acrosses, &vec![&downs; width], &mut vec![]);
}*/

const START: &'static [u8] =
    b",,,,,!,,,,,!,,,,,,
,,,,,!,,,,,!,,,,,,
H,O,O,D,W,I,N,K,E,D,!,,,,,,
,,,,,,!,!,!,,,,!,!,,,
!,!,!,,,,,!,,,,,,,!,!,!
,,,,!,S,H,O,R,T,C,H,A,N,G,E,D
,,,!,,,,,,!,!,!,,,,,
,,,!,,,,,!,,,,,!,,,
,,,,,!,!,!,,,,,,!,,,
H,O,R,N,S,W,O,G,G,L,E,D,!,,,,
!,!,!,,,,,,,!,,,,,!,!,!
,,,!,!,,,,!,!,!,,,,,,
,,,,,,!,B,A,M,B,O,O,Z,L,E,D
,,,,,,!,,,,,!,,,,,
,,,,,,!,,,,,!,,,,,";

const GRID: &'static str =
    "ALLAH█NASA█BAMBOO
GEODE█ARAB█RIBALD
HOODWINKED█ELATED
ANTLER███USA██SOS
███EDIT█ICETEA███
PIED█SHORTCHANGED
ADD█LEAVE███RALLY
RAG█ASIA█PAIL█EEK
CHEAP███EARNS█ACE
HORNSWOGGLED█ANTS
███TEABAG█SIAM███
DOB██HIS███GRACED
OVERDO█BAMBOOZLED
LATINO█AGAR█MOOLA
LLAMAS█GAGA█ANTSY";

fn write_impl() {
    let mut rows = vec![];
    for line in GRID.split('\n') {
        let mut row = vec![];
        for c in line.chars() {
            if c == '█' {
                row.push(Cell::Black);
            } else {
                row.push(Cell::White(Letter::from_unicode(c)));
            }
        }
        rows.push(row);
    }
    let grid = Grid::new((rows[0].len(), rows.len()), |x, y| rows[y][x]);
    println!("{:?}", grid);
    let windows = WindowMap::from_grid(&Grid::new(grid.size(), |x, y| grid[(x, y)] != Cell::Black));
    let mut clues = HashMap::<&str, &str>::new();

    clues.insert("PIED", "Like the proverbial piper");
    clues.insert("DNA", "Twisted pair?");
    clues.insert("PAL", "Alternative to NTSC");
    clues.insert("LATINO", "16.7% of the American population");
    clues.insert("IDAHO", "The Gem State");
    clues.insert("ARES", "Foe of Wonderwoman");
    clues.insert("ELECT", "Opt (to)");
    clues.insert("IRE", "Choler");
    clues.insert("INDIGO", "Infraviolet?");
    clues.insert("TEABAG", "Rude post-victory celebration");
    clues.insert("MAG", "Toner color: abbr.");
    clues.insert("OVERDO", "Cook for 20 minutes, as pasta");
    clues.insert("ADD", "More: abbr.");
    clues.insert("BETA", "Advice, in climbing jargon");
    clues.insert("ARK", "Couple's cruise ship?");
    clues.insert("AIL", "Bedevil");
    clues.insert("EGG", "Urge (on)");
    clues.insert("BREATH", "Form of investiture on Nalthis");
    clues.insert("GRACED", "Adorned");
    clues.insert("OLEO", "Hydrogenated food product");
    clues.insert("ODDS", "What were the ____?");
    clues.insert("GEODE", "Rock formation that starts as a gas bubble");
    clues.insert("HIS", "Label on a towel");
    clues.insert("LEON", "A large gato");
    clues.insert("ADDLED", "Like a brain in love");
    clues.insert("WAHOOS", "Exclamations of joy");
    clues.insert("ARAB", "Desert steed");
    clues.insert("ABDUCT", "Take, as by a UFO");
    clues.insert("MBA", "Degree for CEOs");
    clues.insert("ICETEA", "???");
    clues.insert("DOB", "Important date: abbr");
    clues.insert("CHEAP", "Overpowered, in the 90's");
    clues.insert("RAG", "with \"on\", tease");
    clues.insert("OVA", "Largest human cells");
    clues.insert("RALLY", "Make a comeback, as a military force");
    clues.insert("ANTS", "Pants' contents?");
    clues.insert("EDIT", "Amend");
    clues.insert("AGAR", "Gelatin alternative");
    clues.insert("ASIA", "Home of the Indian elephant");
    clues.insert("AGA", "Ottoman honorific");
    clues.insert("THAI", "Basil variety");
    clues.insert("HORNSWOGGLED", "How you feel after solving this puzzle");
    clues.insert("SOS", "[Help!]");
    clues.insert("EDGER", "Lawnkeeping tool");
    clues.insert("OBI", "Kimono part");
    clues.insert("RIBALD", "Blue");
    clues.insert("ANTLER", "Classic sexual dimorphism feature");
    clues.insert("HOODWINKED", "How you feel after solving this puzzle");
    clues.insert("ACE", "Skilled pilot");
    clues.insert("NASA", "Apollo originator");
    clues.insert("EELS", "Fish caught in pots");
    clues.insert("NAN", "IEEE-754 reflexivity violator");
    clues.insert("DDAY", "Action time");
    clues.insert("SIAM", "Name on old Risk boards");
    clues.insert("EARLS", "Superiors to viscounts");
    clues.insert("USA", "Home of Athens, Berlin, Milan, Palermo, Tripoli, Versailles, and Vienna: abbr");
    clues.insert("BAMBOO", "One of the fasting growing plants in the world");
    clues.insert("ALLAH", "Being with 99 names");
    clues.insert("PAIL", "Bucket");
    clues.insert("PARCH", "Scorch");
    clues.insert("HEWED", "Sawn");
    clues.insert("IRISES", "Organic annuli");
    clues.insert("BRA", "Supporter of women?");
    clues.insert("AROMA", "Bakery attractant");
    clues.insert("LAPSE", "Gap");
    clues.insert("GASBAG", "Yapper");
    clues.insert("ANA", "Serbian tennis player Ivanovic");
    clues.insert("ELATED", "On cloud nine");
    clues.insert("AGHA", "Ottoman honorific");
    clues.insert("BATS", "Spreaders of White-Nose syndrome");
    clues.insert("OVAL", "Egg-like");
    clues.insert("SEC", "Short time, for short");
    clues.insert("MOOLA", "\"Cheddar\"");
    clues.insert("DOLL", "\"It's an action figure, not a ____\"");
    clues.insert("GLEAN", "Reap");
    clues.insert("EARNS", "Reaps");
    clues.insert("ANTSY", "On edge");
    clues.insert("ANT", "Inspiration for a size-warping Marvel hero");
    clues.insert("RIM", "Lid connector");
    clues.insert("BAMBOOZLED", "How you feel after solving this puzzle");
    clues.insert("LOOT", "Reward for killing things, in video games");
    clues.insert("SHORTCHANGED", "How you feel after solving this puzzle");
    clues.insert("EEK", "[A mouse!]");
    clues.insert("GAGA", "Player of the Hotel owner in \"American Horror Story: Hotel\"");
    clues.insert("LLAMAS", "Halfway between a tibetan priest and major fire?");
    clues.insert("DYKES", "Common earthworks");
    clues.insert("SAE", "Standards organization for cars");
    clues.insert("CLOT", "Response to injury, or cause of illness");
    clues.insert("AMAZON", "Origin of Wonderwoman");
    clues.insert("LEAVE", "\"Make like a tree and _____\"");
    let mut clue_map = HashMap::new();
    for (word, clue) in clues {
        clue_map.insert(Word::from_str(word).unwrap(), clue);
    }
    let mut clue_list = vec![];
    for window in windows.windows() {
        let word: Word = window.positions().map(|(x, y)| match grid[(x, y)] {
            Cell::White(Some(x)) => x,
            _ => unreachable!(),
        }).collect();
        clue_list.push((window, clue_map[&word].clone()));
    }
    clue_list.sort_by_key(|(window, clue)| (window.position().0, window.position().1, window.direction()));
    let puzzle = Puzzle {
        preamble: vec![],
        version: *b"1.4\0",
        title: "The First Crossword".to_string(),
        author: "Nathan Dobson".to_string(),
        copyright: "".to_string(),
        grid: Grid::new(grid.size(), |x, y| {
            match grid[(x, y)] {
                Cell::Black => None,
                Cell::White(Some(x)) => Some(PuzzleCell {
                    solution: [x.to_unicode()].iter().cloned().collect(),
                    ..Default::default()
                }),
                _ => panic!(),
            }
        }),
        clues: WindowMap::new(clue_list.into_iter().map(|(window, clue)| (window, clue.to_string())), grid.size()),
        note: "".to_string(),
    };
    let mut new_data: Vec<u8> = vec![];
    puzzle.write_to(&mut new_data).unwrap();
    fs::write("output.puz", &new_data).unwrap();
}

fn edit_impl() -> io::Result<()> {
    let raw = RawScope::new();
    let filename = "output/puzzle.puz";
    let data = fs::read(filename)?;
    let mut puzzle = Puzzle::read_from(&mut data.as_slice())?;
    let mut view = View {
        position: (0, 0),
        direction: Direction::Across,
        editing: true,
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
            let mut play = Play::new(&mut view, &mut puzzle);
            play.do_action(next);
        } else { break; }
    }
    stop_rendering(&mut stdout)?;
    let mut data = vec![];
    puzzle.write_to(&mut &mut data)?;
    fs::write(filename, data)?;
    Ok(())
}

fn main() {
    edit_impl().unwrap();
    //write_impl();
    //search_impl().unwrap();
}

/*fn make_choices(dictionary: &[ScoredWord], grid: &Grid<Cell>, windows: &[Window]) {
    let (window, options) = match windows.iter().enumerate().filter(|(index, window)| {
        for position in window.positions() {
            if grid[position] == Cell::White(None) {
                return true;
            }
        }
        false
    }).map(|(index, window)| {
        let words: Vec<ScoredWord> = dictionary.iter().filter(|word| {
            if word.word.len() != window.length {
                return false;
            }
            for (position, &letter) in window.positions().zip(word.word.iter()) {
                match grid[position] {
                    Cell::White(Some(needed)) => if needed != letter {
                        return false;
                    }
                    _ => {}
                }
            }
            true
        }).cloned().collect();
        (index, words)
    }).min_by_key(|(index, words)| words.len()) {
        None => {
            for y in 0..grid.size().1 {
                for x in 0..grid.size().0 {
                    print!("{}", grid[(x, y)]);
                }
                println!();
            }
            println!();
            return;
        }
        Some(x) => x,
    };

    for y in 0..grid.size().1 {
        for x in 0..grid.size().0 {
            print!("{}", grid[(x, y)]);
        }
        println!();
    }
    println!();

    for option in options {
        let mut grid2 = grid.clone();
        for (position, value) in windows[window].positions().zip(option.word.iter()) {
            grid2[position] = Cell::White(Some(*value));
        }
        make_choices(dictionary, &grid2, windows);
    }
}*/

fn search_impl() -> io::Result<()> {
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .from_reader(START);
    let mut rows = vec![];
    for line in reader.records() {
        let mut row = vec![];
        for cell in line?.iter() {
            row.push(match cell {
                "!" => Cell::Black,
                "" => Cell::White(None),
                letter => Cell::White(Some(Letter::from_unicode(letter.chars().next().unwrap()).unwrap())),
            });
        }
        rows.push(row);
    }
    let grid = Grid::new((rows[0].len(), rows.len()), |x, y| {
        rows[y][x]
    });
    println!("{}", AsciiGrid(&grid));
    let scored_words = ScoredWord::default().unwrap();
    let mut dictionary = scored_words.iter().map(|scored_word| scored_word.word).collect::<Vec<Word>>();
    //dictionary=dictionary[dictionary.]
    //dictionary.push(Word::from_str("bamboozled").unwrap());
    //dictionary.push(Word::from_str("shortchanged").unwrap());
    /*    let windows = windows(&grid, |cell| {
            match cell {
                Cell::Black => true,
                _ => false,
            }
        });
        make_choices(&dictionary, &grid, &windows);*/
    let banned: HashSet<Word> = ["apsis",
        "asdic", "jcloth", "galah", "algin",
        "jinni", "slaty", "jingo", "saiga", "scuta",
        "echt", "concha", "duma", "obeche", "teazel",
        "toerag", "yalta", "howdah", "purdah", "agin", "teehee", "faery", "aubade", "nyala",
        "taenia", "auden", "diable", "craped", "oscan", "halvah",
        "reeve", "dhole", "oca", "balzac", "wahine", "kaons", "medico", "stelae", "asci",
        "anorak", "madedo", "aurous", "dhoti", "GAUR", "AUTEUR", "piaf", "BANZAI", "WABASH", "ERUCT",
        "THRO", "LINAGE", "LABAN", "CHID", "ADDY", "ANOA", "EDO", "BUHL", "BASTS", "EDDO", "IRISED",
        "RAFFIA", "SHARI", "OTIC", "ALTHO", "EFFING", "BROOKE", "BOLL", "BSE", "PEDALO",
        "ELUL", "LARCH", "BORZOI", "DAGO", "LAREDO", "GAOLS", "GIGUE", "TSURIS", "DYADS",
        "STALAG", "SERINE", "BRANDT", "efface", "GAELS", "CRU", "HONIED", "PARSI", "BENZOL", "AACHEN", "DEEDY",
        "CHOREA", "AERY", "CURIO", "RAMEAU", "EFFETE", "EFFUSE", "RIALTO", "ballup", "HAAR",
        "ABATOR", "BAOBAB", "SHILOH", "ATTAR", "ETUI", "REEFY", "RATTAN", "AARHUS", "TENUTO", "NOLL",
        "TWEE", "FOOZLE", "GIBBER", "OUSE", "agm", "gam", "alb", "APPALS",
        "GLOGG", "NEEP", "RIPELY", "ARIL", "ELIJAH", "ADAR", "ALINE", "LIENAL", "EPIZOA", "OGEE"
    ].iter().map(|str| Word::from_str(*str).unwrap()).collect();
//    for &word in banned.iter() {
//        println!("{:?}", word);
//        /println!("{:?} {:?}", word, scored_words.iter().find(|sw| sw.word == word).unwrap().score);
//    }
//    for word in scored_words {
//        if word.score > 30 {
//            println!("{:?}", word.word);
//        }
//    }
    //dictionary.resize((dictionary.len() as f32 * 0.4) as usize, Word::new());
    dictionary.push(Word::from_str("HOODWINKED").unwrap());
    dictionary.push(Word::from_str("SHORTCHANGED").unwrap());
    dictionary.push(Word::from_str("HORNSWOGGLED").unwrap());
    dictionary.push(Word::from_str("BAMBOOZLED").unwrap());
    dictionary.retain(|word| !banned.contains(word));
    {
        let mut search = Search::new(WindowMap::from_grid(&Grid::new(grid.size(), |x, y| grid[(x, y)] != Cell::Black)), &dictionary);

        search.retain(&grid);
        search.refine_all();
        let mut result = None;
        let _ = search.solve(&mut take_one_result(&mut result));
        println!("{:?}", result);
    }


    /*for i in 0..window_set.len() {
        let search = Search {
            windows: window_set.retain(&mut |&window| window != window_set.windows()[i])
        };

        let mut partial = search.start(&dictionary);
        search.retain(&grid, &mut partial);
        search.refine_all(&mut partial);
        if window_set.windows()[i].length < 5 {
            println!("{:?} {:?}", i, window_set.windows()[i]);
            println!("{:?} {:?}", i, search.search_words_by_count(&partial));
            let mut count = 1;
            for p in partial.iter() {
                count *= p.size();
            }
        }
    }*/
    Ok(())
}