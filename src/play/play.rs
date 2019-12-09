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
        if let Some(PuzzleCell { answer, solution, .. }) = &mut self.puzzle.grid[self.view.position] {
            let active_string = if self.view.editing { solution } else { answer };
            *active_string = String::from_utf8(vec![input.to_ascii_uppercase()]).unwrap();
            let window = self.puzzle.clues.window_at(self.view.position, self.view.direction).unwrap();
            match self.view.direction {
                Direction::Across => {
                    self.view.position.0 += 1;
                    if self.view.position.0 >= window.position().0 + window.length() {
                        self.view.position.0 = window.position().0;
                    }
                }
                Direction::Down => {
                    self.view.position.1 += 1;
                    if self.view.position.1 >= window.position().1 + window.length() {
                        self.view.position.1 = window.position().1;
                    }
                }
            }
            self.puzzle_changed = true;
            self.view_changed = true;
        }
    }
    fn do_change_clue(&mut self, delta: isize) {
        match self.puzzle.clues.window_at(self.view.position, self.view.direction) {
            None => {}
            Some(window) => {
                let window = match delta {
                    1 => self.puzzle.clues.next_window(window),
                    -1 => self.puzzle.clues.previous_window(window),
                    _ => panic!(),
                };
                self.view.position = window.position();
                self.view.direction = window.direction();
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
        if let Some(window) = self.puzzle.clues.window_at(self.view.position, self.view.direction) {
            match self.view.direction {
                Direction::Across => {
                    if self.view.position.0 == window.position().0 {
                        self.view.position.0 = window.position().0 + window.length() - 1;
                    } else {
                        self.view.position.0 -= 1;
                    }
                }
                Direction::Down => {
                    if self.view.position.1 == window.position().1 {
                        self.view.position.1 = window.position().1 + window.length() - 1;
                    } else {
                        self.view.position.1 -= 1;
                    }
                }
            }
            match &mut self.puzzle.grid[self.view.position] {
                None => panic!(),
                Some(PuzzleCell { answer, solution, .. }) => {
                    let active_string = if self.view.editing { solution } else { answer };
                    *active_string = "".to_string();
                }
            }
            self.view_changed = true;
            self.puzzle_changed = true;
        }
    }
}