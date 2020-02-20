use super::puzzle::PuzzleCell;
use super::puzzle::Puzzle;
use super::puzzle::View;
use crate::core::puzzle::{Direction, WindowMap, Cell, Window};
use crate::util::grid::Grid;
use crate::play::puzzle::Mode;
use crate::core::letter::Letter;
use crate::fill::dictionary::EditedDictionary;
use itertools::Itertools;
use crate::core::word::Word;
use crate::fill::search::{Search, take_one_result};
use std::iter;

pub enum Action {
    MoveUp,
    MoveDown,
    MoveRight,
    MoveLeft,
    Type { letter: u8 },
    ChangeClue { change: isize },
    Delete,
    ChangeColor,
    Generate,
    Accept,
    Reject,
    TogglePencil,
    ToggleEditClue,
}

pub struct Play<'a> {
    view: &'a mut View,
    puzzle: &'a mut Puzzle,
    dictionary: Option<&'a mut EditedDictionary>,
    view_changed: bool,
    puzzle_changed: bool,
}

fn decrease(state: &mut usize) -> bool {
    if *state > 0 {
        *state -= 1;
        true
    } else {
        false
    }
}

fn increase(state: &mut usize, limit: usize) -> bool {
    if *state + 1 < limit {
        *state += 1;
        true
    } else {
        false
    }
}

impl<'a> Play<'a> {
    pub fn new(view: &'a mut View, puzzle: &'a mut Puzzle, dictionary: Option<&'a mut EditedDictionary>) -> Self {
        Play {
            view,
            puzzle,
            dictionary,
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
            Action::Delete => self.do_delete(),
            Action::ChangeColor => self.change_color(),
            Action::Generate => self.generate(),
            Action::Accept => self.accept(),
            Action::Reject => self.reject(),
            Action::TogglePencil => self.toggle_pencil(),
            Action::ToggleEditClue => self.toggle_edit_clue(),
        }
    }
    fn do_move_up(&mut self) {
        if match &mut self.view.mode {
            Mode::EditingClue { cursor } => false,
            _ => decrease(&mut self.view.position.1)
        } {
            self.view_changed = true;
        }
    }
    fn do_move_down(&mut self) {
        if match &mut self.view.mode {
            Mode::EditingClue { cursor } => false,
            _ => increase(&mut self.view.position.1, self.puzzle.grid.size().1)
        } {
            self.view_changed = true;
        }
    }

    fn do_move_right(&mut self) {
        let window = self.get_current_window();
        if match &mut self.view.mode {
            Mode::EditingClue { cursor } =>
                increase(cursor, self.puzzle.clues[window.unwrap()].len() + 1),
            _ => increase(&mut self.view.position.0, self.puzzle.grid.size().0),
        } {
            self.view_changed = true;
        }
    }

    fn do_move_left(&mut self) {
        if match &mut self.view.mode {
            Mode::EditingClue { cursor } => decrease(cursor),
            _ => decrease(&mut self.view.position.0),
        } {
            self.view_changed = true;
        }
    }

    fn do_type(&mut self, input: u8) {
        let window = self.get_current_window();
        match &mut self.view.mode {
            Mode::EditingClue { cursor } =>
                Self::do_type_in_clue(
                    cursor,
                    &mut self.puzzle.clues[window.unwrap()],
                    input),
            _ => {
                match input {
                    b' ' => self.do_change_direction(),
                    _ => self.do_type_in_grid(input)
                }
            }
        }
    }

    fn do_type_in_clue(cursor: &mut usize, clue: &mut String, input: u8) {
        clue.insert(*cursor, input as char);
        *cursor += 1;
    }

    fn do_type_in_grid(&mut self, input: u8) {
        if let Some(PuzzleCell { answer, solution, pencil, .. }) = &mut self.puzzle.grid[self.view.position] {
            let active_string = if self.view.mode == Mode::Solving { answer } else { solution };
            *pencil = self.view.pencil;
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
        match &mut self.view.mode {
            Mode::EditingClue { cursor } => *cursor = 0,
            _ => {}
        }
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
        match self.view.mode {
            Mode::EditingClue { cursor } => {
                self.do_type(b' ')
            }
            _ => {
                self.view.direction = match self.view.direction {
                    Direction::Across => Direction::Down,
                    Direction::Down => Direction::Across,
                };
                self.view_changed = true;
            }
        }
    }

    fn do_delete(&mut self) {
        let window = self.get_current_window();
        match &mut self.view.mode {
            Mode::EditingClue { cursor } =>
                Self::do_delete_in_clue(
                    cursor,
                    &mut self.puzzle.clues[window.unwrap()]),
            _ => self.do_delete_in_grid()
        }
    }

    fn do_delete_in_clue(cursor: &mut usize, clue: &mut String) {
        if *cursor > 0 {
            clue.remove(*cursor - 1);
            *cursor -= 1;
        }
    }

    fn do_delete_in_grid(&mut self) {
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
                    let active_string = if self.view.mode == Mode::Solving { answer } else { solution };
                    *active_string = "".to_string();
                }
            }
            self.view_changed = true;
            self.puzzle_changed = true;
        }
    }

    fn change_color(&mut self) {
        if self.view.mode == Mode::Solving {
            return;
        }
        if self.puzzle.grid[self.view.position].is_some() {
            self.puzzle.grid[self.view.position] = None;
        } else {
            self.puzzle.grid[self.view.position] = Some(PuzzleCell::default());
        }
        let new_windows =
            WindowMap::from_grid(
                &Grid::new(self.puzzle.grid.size(),
                           |x, y| self.puzzle.grid[(x, y)].is_some()));
        self.puzzle.clues = WindowMap::new(new_windows.windows().map(|window| {
            (window, self.puzzle.clues.get(window).map_or("".to_string(), |clue| clue.clone()))
        }), self.puzzle.grid.size());
    }

    fn generate(&mut self) {
        let grid = Grid::new(
            self.puzzle.grid.size(),
            |x, y|
                match &self.puzzle.grid[(x, y)] {
                    None => Cell::Black,
                    Some(c) => {
                        if c.pencil {
                            Cell::White(None)
                        } else {
                            Cell::White(Letter::from_str(&c.solution))
                        }
                    }
                },
        );
        let dictionary = self.dictionary.as_ref().unwrap().build();
        let mut search = Search::new(
            WindowMap::from_grid(
                &Grid::new(grid.size(), |x, y| grid[(x, y)] != Cell::Black)), &dictionary);
        search.retain(&grid);
        search.refine_all();
        eprintln!("{:?}", search);
        let mut result = None;
        let _ = search.solve(&mut take_one_result(&mut result));
        if let Some(solution) = result {
            for y in 0..grid.size().1 {
                for x in 0..grid.size().0 {
                    if let Some(cell) = self.puzzle.grid[(x, y)].as_mut() {
                        let new = iter::once(solution.letter_set((x, y)).unwrap().unique().unwrap().to_unicode()).collect();
                        if cell.solution != new {
                            cell.solution = new;
                            cell.pencil = true;
                            for &direction in &[Direction::Across, Direction::Down] {
                                if let Some(window) = self.puzzle.clues.window_at((x, y), direction) {
                                    self.puzzle.clues[window] = format!("AUTO: {}", self.get_solution_word(window).to_unicode());
                                }
                            }
                        }
                    }
                }
            }
        } else {
            eprintln!("No solution");
        }
    }

    fn get_current_window(&self) -> Option<Window> {
        self.puzzle.clues
            .window_at(self.view.position, self.view.direction)
    }

    fn get_solution_word(&self, window: Window) -> Word {
        Word::from_str(window
            .positions()
            .map(|position|
                self.puzzle.grid[position].as_ref().unwrap().solution.chars())
            .flatten()
            .collect::<String>()
            .as_str()).unwrap()
    }

    fn goto_unknown(&mut self) {
        if let Some(window) = self.puzzle.clues.windows().find(|&window| {
            self.dictionary.as_ref().unwrap().status(self.get_solution_word(window)) == None
        }) {
            self.view.position = window.position();
            self.view.direction = window.direction();
            self.view_changed = true;
        }
    }

    fn accept(&mut self) {
        let word = self.get_solution_word(self.get_current_window().unwrap());
        self.dictionary.as_mut().unwrap().set_status(word, Some(true));
        self.goto_unknown();
        self.view_changed = true;
        self.puzzle_changed = true;
    }

    fn reject(&mut self) {
        let word = self.get_solution_word(self.get_current_window().unwrap());
        self.dictionary.as_mut().unwrap().set_status(word, Some(false));
        self.generate();
        self.goto_unknown();
        self.view_changed = true;
        self.puzzle_changed = true;
    }

    fn toggle_pencil(&mut self) {
        self.view.pencil = !self.view.pencil;
        self.view_changed = true;
    }

    fn toggle_edit_clue(&mut self) {
        match self.view.mode {
            Mode::Solving => {}
            Mode::Editing => self.view.mode = Mode::EditingClue { cursor: 0 },
            Mode::EditingClue { .. } => self.view.mode = Mode::Editing,
        }
        self.view_changed = true;
    }
}