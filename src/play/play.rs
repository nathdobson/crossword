use super::puzzle::PuzzleCell;
use super::puzzle::Puzzle;
use super::puzzle::View;
use crate::core::puzzle::Direction;

pub enum Action {
    MoveUp,
    MoveDown,
    MoveRight,
    MoveLeft,
    Type { letter: u8 },
    ChangeClue { change: isize },
    ChangeDirection,
    Delete,
}

pub struct Play<'a> {
    view: &'a mut View,
    puzzle: &'a mut Puzzle,
    view_changed: bool,
    puzzle_changed: bool,
}

impl<'a> Play<'a> {
    pub fn new(view: &'a mut View, puzzle: &'a mut Puzzle) -> Self {
        Play {
            view: view,
            puzzle: puzzle,
            view_changed: false,
            puzzle_changed: false,
        }
    }
    pub fn view_changed(&self) -> bool {
        self.view_changed
    }
    pub fn puzzle_changed(&self) -> bool {
        self.puzzle_changed
    }
    pub fn do_action(&mut self, action: Action) {
        match action {
            Action::MoveUp => self.do_move_up(),
            Action::MoveDown => self.do_move_down(),
            Action::MoveRight => self.do_move_right(),
            Action::MoveLeft => self.do_move_left(),
            Action::Type { letter } => self.do_type(letter),
            Action::ChangeClue { change } => self.do_change_clue(change),
            Action::ChangeDirection => self.do_change_direction(),
            Action::Delete => self.do_delete(),
        }
    }
    fn do_move_up(&mut self) {
        if self.view.position.1 > 0 {
            self.view.position.1 -= 1;
            self.view_changed = true;
        }
    }
    fn do_move_down(&mut self) {
        if self.view.position.1 < self.puzzle.grid.size().1 - 1 {
            self.view.position.1 += 1;
            self.view_changed = true;
        }
    }
    fn do_move_right(&mut self) {
        if self.view.position.0 < self.puzzle.grid.size().0 - 1 {
            self.view.position.0 += 1;
            self.view_changed = true;
        }
    }
    fn do_move_left(&mut self) {
        if self.view.position.0 > 0 {
            self.view.position.0 -= 1;
            self.view_changed = true;
        }
    }
    fn do_type(&mut self, input: u8) {
        match &mut self.puzzle.grid[self.view.position] {
            PuzzleCell::Black => {}
            PuzzleCell::White { across_clue, down_clue, answer, .. } => {
                *answer = Some(String::from_utf8(vec![input.to_ascii_uppercase()]).unwrap());
                match self.view.direction {
                    Direction::Across => {
                        let clue = &self.puzzle.clues[*across_clue];
                        self.view.position.0 += 1;
                        if self.view.position.0 >= clue.window.position().0 + clue.window.length() {
                            self.view.position.0 = clue.window.position().0;
                        }
                    }
                    Direction::Down => {
                        let clue = &self.puzzle.clues[*down_clue];
                        self.view.position.1 += 1;
                        if self.view.position.1 >= clue.window.position().1 + clue.window.length() {
                            self.view.position.1 = clue.window.position().1;
                        }
                    }
                }
                self.puzzle_changed = true;
                self.view_changed = true;
            }
        }
    }
    fn do_change_clue(&mut self, delta: isize) {
        match self.puzzle.get_clue(self.view) {
            None => {}
            Some(clue) => {
                let mut new_clue = clue as isize + delta;
                if new_clue >= self.puzzle.clues.len() as isize {
                    new_clue = 0;
                }
                if new_clue < 0 {
                    new_clue = (self.puzzle.clues.len() - 1) as isize;
                }
                let clue = new_clue as usize;
                self.view.position = self.puzzle.clues[clue].window.position();
                self.view.direction = self.puzzle.clues[clue].window.direction();
                self.view_changed = true;
            }
        }
    }
    fn do_change_direction(&mut self) {
        self.view.direction = match self.view.direction {
            Direction::Across => Direction::Down,
            Direction::Down => Direction::Across,
        };
        self.view_changed = true;
    }
    fn do_delete(&mut self) {
        if match &mut self.puzzle.grid[self.view.position] {
            PuzzleCell::Black => false,
            PuzzleCell::White { across_clue, down_clue, answer, .. } => {
                match self.view.direction {
                    Direction::Across => {
                        let clue = &self.puzzle.clues[*across_clue];
                        if self.view.position.0 == clue.window.position().0 {
                            self.view.position.0 = clue.window.position().0 + clue.window.length() - 1;
                        } else {
                            self.view.position.0 -= 1;
                        }
                    }
                    Direction::Down => {
                        let clue = &self.puzzle.clues[*down_clue];
                        if self.view.position.1 == clue.window.position().1 {
                            self.view.position.1 = clue.window.position().1 + clue.window.length() - 1;
                        } else {
                            self.view.position.1 -= 1;
                        }
                    }
                }
                true
            }
        } {
            match &mut self.puzzle.grid[self.view.position] {
                PuzzleCell::Black => panic!(),
                PuzzleCell::White { across_clue, down_clue, answer, .. } => {
                    *answer = None;
                }
            }
            self.view_changed = true;
            self.puzzle_changed = true;
        }
    }
}