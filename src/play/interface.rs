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
use crate::core::puzzle::Window;
use termios::cfmakeraw;
use termios::{Termios, TCSANOW, ECHO, ICANON, tcsetattr};
use crate::play::puzzle::Mode::Solving;
use crate::play::puzzle::Mode;

pub struct RawScope {
    termios: Termios
}

impl RawScope {
    pub fn new() -> Self {
        let termios = Termios::from_fd(0).unwrap();
        let mut new_termios = termios.clone();
        cfmakeraw(&mut new_termios);
        tcsetattr(0, TCSANOW, &mut new_termios).unwrap();
        RawScope {
            termios: termios
        }
    }
}

impl Drop for RawScope {
    fn drop(&mut self) {
        tcsetattr(0, TCSANOW, &self.termios).unwrap();
    }
}

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
x xx x
x    x
x    x
x    x
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
x    x
x    x
x    x
x xx x
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
    fn render_cell(&mut self, x: usize, y: usize, dy: usize, active_clue: Option<Window>) -> io::Result<()> {
        match &self.puzzle.grid[(x, y)] {
            None => {
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
                    if dy == 0 && y > 0 && self.puzzle.grid[(x, y - 1)].is_some() {
                        '▇'
                    } else {
                        '█'
                    };
                write!(self.output, "\x1B[48;5;{};38;5;{}m{}\x1B[0m", background, foreground, iter::repeat(c).take(CELL_WIDTH).collect::<String>())?;
            }
            Some(PuzzleCell { answer, solution, circled, pencil, .. }) => {
                let background = if self.view.position == (x, y) {
                    11
                } else if active_clue.map(|active_clue| active_clue.offset((x, y)).is_some()).unwrap_or(false) {
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
                let contents_string = if self.view.mode == Solving {
                    answer
                } else {
                    solution
                };
                let contents = if contents_string.chars().count() == 1 {
                    let grid = draw_letter(contents_string.chars().next().unwrap() as u8);
                    (0..CELL_WIDTH).map(|dx| grid[(dx, dy)]).collect::<String>()
                } else if contents_string == "" {
                    iter::repeat(' ').take(CELL_WIDTH).collect::<String>()
                } else {
                    unimplemented!();
                };
                let code = if *circled {
                    format!("\x1b[4m{}\x1b[0m", contents)
                    //\u{032e}
                } else {
                    format!("{}", contents)
                };
                let foreground = if *pencil { 244 } else { 16 };
                write!(self.output, "\x1B[48;5;{};38;5;{}m{}\x1B[0m", background, foreground, code)?;
            }
        }
        Ok(())
    }
    pub fn render(&mut self) -> io::Result<()> {
        write!(self.output, "\x1b]0;{}\x07", self.puzzle.title)?;
        write!(self.output, "\x1B[H\x1B[J")?;


        let active_clue = self.puzzle.clues.window_at(self.view.position, self.view.direction);
        for y in 0..self.puzzle.grid.size().1 {
            for dy in 0..CELL_HEIGHT {
                //write!(self.output, "\x1B#{}", half);
                for x in 0..self.puzzle.grid.size().0 {
                    self.render_cell(x, y, dy, active_clue)?;
                }
                write!(self.output, "\r\n")?;
            }
        }
        for half in &[3, 4] {
            write!(self.output, "\x1B#{}", half)?;
            if self.view.pencil {
                write!(self.output, "\x1B[7m")?;
            }
            write!(self.output, "✎")?;
            if self.view.pencil {
                write!(self.output, "\x1B[0m")?;
            }
            write!(self.output, "\r\n")?;
        }

        if let Some(active_clue) = active_clue {
            let mut cursor = match self.view.mode {
                Mode::EditingClue { cursor } => Some(cursor),
                _ => None
            };
            let with_space = format!("{} ", &self.puzzle.clues[active_clue]);
            for line in break_lines(&with_space, 50) {
                for half in &[3, 4] {
                    write!(self.output, "\x1B#{}", half)?;
                    match cursor {
                        Some(c) if c < line.len() => {
                            write!(self.output, "{}\x1B[7m{}\x1B[0m{}",
                                   &line[0..c],
                                   &line[c..c + 1],
                                   &line[c + 1..])?;
                        }
                        _ => {
                            write!(self.output, "{}", line)?;
                        }
                    }
                    write!(self.output, "\r\n")?;
                }
                cursor = cursor.and_then(|x| x.checked_sub(line.len()));
            }
        };

        if self.view.mode == Mode::Solving {
            let solved = self.puzzle.grid.iter().all(|cell| {
                if let Some(PuzzleCell { answer, solution, .. }) = cell {
                    if solution != answer {
                        return false;
                    }
                }
                true
            });
            if solved {
                write!(self.output, "\r\n\r\n")?;
                for half in &[3, 4] {
                    write!(self.output, "\x1B#{}{}\r\n", half, "🎉🎉🎉🎉CONGRATULATIONS🎉🎉🎉🎉")?;
                }
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
                //x @ b'A'..=b'Z' | x @ b'a'..=b'z' | b' ' => {
                x @ b' '..=b'~' => {
                    return Ok(Some(Action::Type { letter: x }));
                }
                13 | 9 => {
                    return Ok(Some(Action::ChangeClue { change: 1 }));
                }
                23 => {
                    return Ok(Some(Action::ChangeColor));
                }
                7 => {
                    return Ok(Some(Action::Generate));
                }
                1 => {
                    return Ok(Some(Action::Reject));
                }
                19 => {
                    return Ok(Some(Action::Accept));
                }
                127 => {
                    return Ok(Some(Action::Delete));
                }
                16 => {
                    return Ok(Some(Action::TogglePencil));
                }
                5 => {
                    return Ok(Some(Action::ToggleEditClue));
                }
                x => {
                    eprintln!("unknown = {}", x);
                }
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

