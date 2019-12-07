use std::io::Write;
use std::io;
use super::puzzle::PuzzleCell;
use super::puzzle::View;
use super::puzzle::Puzzle;
use std::io::Read;
use super::play::Action;
use byteorder::ReadBytesExt;
use std::convert::TryFrom;
use std::iter;
use crate::util::grid::Grid;
use crate::util::lines::break_lines;

pub fn start_rendering(output: &mut dyn Write) -> io::Result<()> {
    write!(output, "\x1B[?1049h\x1B[?25l")?;
    output.flush()?;
    Ok(())
}

pub fn stop_rendering(output: &mut dyn Write) -> io::Result<()> {
    write!(output, "\x1B[?1049l\x1B[?25h")?;
    output.flush()?;
    Ok(())
}

pub struct TerminalOutput<'a> {
    pub output: &'a mut dyn Write,
    pub view: &'a View,
    pub puzzle: &'a Puzzle,
}

const CELL_WIDTH: usize = 5;
const CELL_HEIGHT: usize = 3;
const OFFSET_X: usize = 2;
const OFFSET_Y: usize = 1;

static ALPHABET: &[&str] = &["
xxxxxx
xx  xx
xxxxxx
xx  xx
xx  xx
", "
xxxxx
xx  xx
xxxxx
xx  xx
xxxxx
", "
 xxxxx
xx
xx
xx
 xxxxx
", "
xxxxx
xx  xx
xx  xx
xx  xx
xxxxx
", "
xxxxxx
xx
xxxxxx
xx
xxxxxx
", "
xxxxxx
xx
xxxxx
xx
xx
", "
xxxxxx
xx
xx xxx
xx  xx
xxxxxx
", "
xx  xx
xx  xx
xxxxxx
xx  xx
xx  xx
", "
xxxxxx
  xx
  xx
  xx
xxxxxx
", "
xxxxxx
  xx
  xx
  xx
xxxx
", "
xx  xx
xx xx
xxxx
xx xx
xx  xx
", "
xx
xx
xx
xx
xxxxxx
", "
xx  xx
xxxxxx
xx  xx
xx  xx
xx  xx
", "
xx  xx
xxx xx
xxxxxx
xx xxx
xx  xx
", "
 xxxx
xx  xx
xx  xx
xx  xx
 xxxx
", "
xxxxx
xx  xx
xxxxx
xx
xx
", "
 xxxx
xx  xx
xx  xx
xx xx
 xxx x
", "
xxxxx
xx  xx
xxxxx
xx xx
xx  xx
", "
 xxxxx
xx
 xxxx
    xx
xxxxx
", "
xxxxxx
  xx
  xx
  xx
  xx
", "
xx  xx
xx  xx
xx  xx
xx  xx
xxxxxx
", "
x    x
xx  xx
xx  xx
 x  x
 xxxx
", "
xx  xx
xx  xx
xx  xx
xxxxxx
xx  xx
", "
xx  xx
 xxxx
  xx
 xxxx
xx  xx
", "
xx  xx
xx  xx
 xxxx
  xx
  xx
", "
xxxxxx
   xx
  xx
 xx
xxxxxx
"];

fn draw_box(c11: bool, c21: bool, c12: bool, c22: bool) -> char {
    match (c11, c21, c12, c22) {
        (false, false, false, false) => ' ',

        (true, false, false, false) => '▘',
        (false, true, false, false) => '▝',
        (false, false, true, false) => '▖',
        (false, false, false, true) => '▗',

        (true, true, false, false) => '▀',
        (false, false, true, true) => '▄',
        (true, false, true, false) => '▌',
        (false, true, false, true) => '▐',

        (true, false, false, true) => '▚',
        (false, true, true, false) => '▞',

        (false, true, true, true) => '▟',
        (true, false, true, true) => '▙',
        (true, true, false, true) => '▜',
        (true, true, true, false) => '▛',

        (true, true, true, true) => '█',
    }
}

fn draw_letter(letter: u8) -> Grid<char> {
    let raw = ALPHABET[(letter.to_ascii_uppercase() - b'A') as usize];
    let raw = &raw[1..raw.len() - 1];
    let lines = raw.split('\n').collect::<Vec<_>>();
    Grid::new((CELL_WIDTH, CELL_HEIGHT), |x, y| {
        let l1: &str = if y * 2 >= OFFSET_Y { lines.get(y * 2 - OFFSET_Y).unwrap_or(&"") } else { &"" };
        let l2: &str = lines.get(y * 2).unwrap_or(&"");
        fn get_pixel(l: &str, x: usize) -> bool {
            if x >= OFFSET_X {
                l.chars().nth(x - OFFSET_X) == Some('x')
            } else { false }
        }
        let c11 = get_pixel(&l1, x * 2);
        let c21 = get_pixel(&l1, x * 2 + 1);
        let c12 = get_pixel(&l2, x * 2);
        let c22 = get_pixel(&l2, x * 2 + 1);
        //println!("{} {} {} {} {} {}", x, y, c11, c21, c12, c22);
        draw_box(c11, c21, c12, c22)
    })
}

impl<'a> TerminalOutput<'a> {
    fn render_cell(&mut self, x: usize, y: usize, dy: usize, active_clue: Option<usize>) -> io::Result<()> {
        match &self.puzzle.grid[(x, y)] {
            PuzzleCell::Black => {
                let foreground: i32 = if self.view.position == (x, y) {
                    3
                } else {
                    16
                };
                let background = if (x + y) % 2 == 0 {
                    252
                } else {
                    15
                };
                let c =
                    if dy == 0 && y > 0 && match self.puzzle.grid[(x, y - 1)] {
                        PuzzleCell::Black => false,
                        _ => true
                    } {
                        '▇'
                    } else {
                        '█'
                    };
                write!(self.output, "\x1B[48;5;{};38;5;{}m{}\x1B[0m", background, foreground, iter::repeat(c).take(CELL_WIDTH).collect::<String>())?;
            }
            PuzzleCell::White { across_clue, down_clue, answer, circled, .. } => {
                let background = if self.view.position == (x, y) {
                    11
                } else if Some(*across_clue) == active_clue || Some(*down_clue) == active_clue {
                    if (x + y) % 2 == 0 {
                        51
                    } else {
                        14
                    }
                } else {
                    if (x + y) % 2 == 0 {
                        15
                    } else {
                        252
                    }
                };
                let contents = match answer {
                    None => iter::repeat(' ').take(CELL_WIDTH).collect::<String>(),
                    Some(c) => {
                        let grid = draw_letter(c.chars().next().unwrap() as u8);
                        (0..CELL_WIDTH).map(|dx| grid[(dx, dy)]).collect::<String>()
                    }
                };
                let code = if *circled {
                    format!("\x1b[4m{}\x1b[0m", contents)
                    //\u{032e}
                } else {
                    format!("{}", contents)
                };
                write!(self.output, "\x1B[48;5;{};38;5;16m{}\x1B[0m", background, code)?;
            }
        }
        Ok(())
    }
    pub fn render(&mut self) -> io::Result<()> {
        write!(self.output, "\x1b]0;{}\x07", self.puzzle.title)?;
        write!(self.output, "\x1B[H\x1B[J")?;


        let active_clue = self.puzzle.get_clue(self.view);
        for y in 0..self.puzzle.grid.size().1 {
            for dy in 0..CELL_HEIGHT {
                //write!(self.output, "\x1B#{}", half);
                for x in 0..self.puzzle.grid.size().0 {
                    self.render_cell(x, y, dy, active_clue)?;
                }
                write!(self.output, "\r\n")?;
            }
        }

        if let Some(active_clue) = active_clue {
            for line in break_lines(&self.puzzle.clues[active_clue].clue, 50) {
                for half in &[3, 4] {
                    write!(self.output, "\x1B#{}{}\r\n", half, line)?;
                }
            }
        };

        let mut solved = true;
        for cell in self.puzzle.grid.iter() {
            match cell {
                PuzzleCell::White { answer, solution, .. } => {
                    if Some(solution) != answer.as_ref() {
                        solved = false;
                    }
                }
                _ => {}
            }
        }
        if solved {
            write!(self.output, "\r\n\r\n")?;
            for half in &[3, 4] {
                write!(self.output, "\x1B#{}{}\r\n", half, "🎉🎉🎉🎉CONGRATULATIONS🎉🎉🎉🎉")?;
            }
        }

        self.output.flush()?;

        Ok(())
    }
}

pub struct TerminalInput<'a> {
    pub input: &'a mut dyn Read,
}

impl<'a> TerminalInput<'a> {
    pub fn read_event(&mut self) -> io::Result<Option<Action>> {
        loop {
            match self.input.read_u8()? {
                0x3 | 0x4 => return Ok(None),
                0x1B => match self.input.read_u8()? {
                    b'[' => match self.input.read_u8()? {
                        b'A' => {
                            return Ok(Some(Action::MoveUp));
                        }
                        b'B' => {
                            return Ok(Some(Action::MoveDown));
                        }
                        b'C' => {
                            return Ok(Some(Action::MoveRight));
                        }
                        b'D' => {
                            return Ok(Some(Action::MoveLeft));
                        }
                        b'Z' => {
                            return Ok(Some(Action::ChangeClue { change: -1 }));
                        }
                        z => {}
                    }
                    y => {}
                }
                x @ b'A'..=b'Z' | x @ b'a'..=b'z' => {
                    return Ok(Some(Action::Type { letter: x }));
                }
                b' ' => {
                    return Ok(Some(Action::ChangeDirection));
                }
                13 | 9 => {
                    return Ok(Some(Action::ChangeClue { change: 1 }));
                }
                127 => {
                    return Ok(Some(Action::Delete));
                }
                x => {}
            }
        }
    }
}

#[test]
fn test_letter() {
    for letter in b'A'..=b'Z' {
        let grid = draw_letter(letter);
        for y in 0..grid.size().1 {
            for x in 0..grid.size().0 {
                print!("{}", grid[(x, y)]);
            }
            println!();
        }
        //println!();
    }
}

