use crate::core::puzzle::{WindowMap, Direction, AsciiGrid, Window, Cell};
use crate::fill::word_set::WordSet;
use std::collections::{HashSet, HashMap};
use crate::core::letter::{Letter, LetterSet};
use crate::core::word::Word;
use crate::util::grid::Grid;
use rand::distributions::{Bernoulli, Distribution};
use crate::util::graph::{Graph, stoer_wagner};
use std::fmt;
use rand::Rng;
use crate::util::product::CartesianProduct;

#[derive(Clone)]
pub struct Search {
    pub sets: WindowMap<WordSet>,
}

pub struct Canceled;

type Result = std::result::Result<(), Canceled>;

pub fn take_one_result<'a, T>(result: &'a mut Option<T>) -> impl 'a + FnMut(T) -> Result {
    move |value| {
        *result = Some(value);
        Err(Canceled)
    }
}

pub fn take_all_results<'a, T>(result: &'a mut Vec<T>) -> impl 'a + FnMut(T) -> Result {
    move |value| {
        result.push(value);
        Ok(())
    }
}

impl fmt::Debug for Search {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, (window, set)) in self.sets.iter().enumerate() {
            if set.size() == 1 {
                write!(f, "{:?} ", set)?;
            }
        }
        writeln!(f)?;
        write!(f, "{}", AsciiGrid(&self.finish()))?;
        Ok(())
    }
}

impl Search {
    pub fn new(windows: WindowMap<()>, words: &[Word]) -> Self {
        Search {
            sets: WindowMap::new(
                windows
                    .windows().map(
                    |window| {
                        (window, WordSet::from_words(words, window.length()))
                    }
                ), windows.grid_size())
        }
    }

    pub fn finish(&self) -> Grid<Cell> {
        let mut result = Grid::new(self.sets.grid_size(), |x, y| {
            Cell::Black
        });
        for (window, set) in self.sets.iter() {
            for (index, position) in window.positions().enumerate() {
                let letters = set.letters(index);
                match result[position] {
                    Cell::Black => result[position] = Cell::White(letters.unique()),
                    Cell::White(None) => result[position] = Cell::White(letters.unique()),
                    Cell::White(Some(x)) => {}
                }
            }
        }
        result
    }

    pub fn retain(&mut self, grid: &Grid<Cell>) {
        for (window, set) in self.sets.iter_mut() {
            set.retain(|word| {
                for (position, &letter) in window.positions().zip(word.iter()) {
                    match grid[position] {
                        Cell::White(Some(wanted)) => if letter != wanted {
                            return false;
                        }
                        _ => {}
                    }
                }
                true
            });
        }
    }

    pub fn refine_all(&mut self) {
        self.refine(self.sets.windows().collect());
    }

    pub fn refine_one(&mut self, window: Window) {
        let mut dirty = HashSet::new();
        dirty.insert(window);
        self.refine(dirty);
    }

    fn refine(&mut self, mut dirty: HashSet<Window>) {
        while let Some(&window) = dirty.iter().next() {
            for (offset, position) in window.positions().enumerate() {
                if let Some(perp) = self.sets.window_at(position, window.direction().perpendicular()) {
                    let perp_offset = perp.offset(position).unwrap();
                    let letters = self.sets[window].letters(offset);
                    let perp_letters = self.sets[perp].letters(perp_offset);
                    if !perp_letters.is_subset(letters) {
                        self.sets[perp].retain(|word| {
                            letters.get(word[perp_offset])
                        });
                        dirty.insert(perp);
                    }
                }
            }
            dirty.remove(&window);
        }
    }

    /*pub fn search_squares(&self, sets: &Vec<WordSet>, positions: &[(usize, usize)]) -> Option<Vec<WordSet>> {
        if sets.iter().any(|set| set.size() == 0) {
            return None;
        }
        if positions.len() == 0 {
            return Some(sets.clone());
        }
        let position = positions[0];
        let window = self.windows.window_at(position, Direction::Across).or(self.windows.window_at(position, Direction::Down)).unwrap();
        let offset = self.windows[window].offset(position);
        let mut options: Vec<Letter> = Letter::all().filter(|&letter| { sets[window].letters(offset).get(letter) }).collect();
        options.sort_by_key(|&letter| -(sets[window].count(offset, letter) as isize));
        for letter in options {
            let mut sets2 = (*sets).clone();
            sets2[window].retain(|word| {
                word[offset] == letter
            });
            self.refine_one(&mut sets2, window);
            if let Some(result) = self.search_squares(&sets2, &positions[1..]) {
                return Some(result);
            }
        }
        None
    }*/

    /*pub fn search_words(&self, sets: &Vec<WordSet>, windows: &[usize]) -> Option<Vec<WordSet>> {
        if sets.iter().any(|set| set.size() == 0) {
            return None;
        }
        if windows.len() == 0 {
            return Some(sets.clone());
        }
        let window = windows[0];
        for &word in sets[window].words() {
            let mut search2 = self.clone();
            search2.sets[window] = WordSet::from_words(&[word], word.len());
            for (window2, set) in search2.sets.iter_mut().enumerate() {
                if window2 != window {
                    set.retain(|word2| word2 != word);
                }
            }

            search2.refine_one(window);
            if let Some(result) = search2.search_words(&windows[1..]) {
                return Some(result);
            }
        }
        None
    }*/


    pub fn filter(&self, condition: &mut dyn FnMut(Window) -> bool) -> Search {
        Search {
            sets: WindowMap::new(
                self.sets.iter()
                    .filter(|(window, set)| condition(*window))
                    .map(|(window, set)| (window, set.clone()))
                ,
                self.sets.grid_size())
        }
    }

    pub fn split_cells(&self) -> Option<(Vec<(usize, usize)>, [Search; 2])> {
        let mut graph = Graph::new();
        let mut vertices = HashMap::new();
        for (window, set) in self.sets.iter() {
            if set.size() > 1 {
                vertices.insert(window, graph.add_vertex(window));
            }
        }
        if vertices.len() < 2 {
            return None;
        }
        for (window, set) in self.sets.iter() {
            for position in window.positions() {
                if let Some(intersection) = self.sets.window_at(position, window.direction().perpendicular()) {
                    if let (Some(&window_vertex), Some(&intersection_vertex))
                    = (vertices.get(&window), vertices.get(&intersection)) {
                        graph.add_edge(
                            window_vertex,
                            intersection_vertex,
                            position);
                    }
                }
            }
        }
        let (cost, split) = stoer_wagner(&graph);
        let split_set: HashSet<Window> = split.iter().map(|vertex| *graph.label(*vertex)).collect();
        let mut overlap = vec![];
        for x in 0..self.sets.grid_size().0 {
            for y in 0..self.sets.grid_size().1 {
                match (self.sets.window_at((x, y), Direction::Across),
                       self.sets.window_at((x, y), Direction::Down)) {
                    (Some(across), Some(down)) => {
                        if vertices.contains_key(&across)
                            && vertices.contains_key(&down)
                            && split_set.contains(&across) != split_set.contains(&down) {
                            overlap.push((x, y));
                        }
                    }
                    _ => {}
                }
            }
        }
        assert_eq!(cost, overlap.len());
        Some((overlap,
              [self.filter(&mut |window| split_set.contains(&window)),
                  self.filter(&mut |window| !split_set.contains(&window))]
        ))
    }


    pub fn solve_direct(&self, callback: &mut dyn FnMut(Search) -> Result) -> Result {
        let window =
            match self.sets.iter()
                .filter(|(window, set)| set.size() > 1)
                .min_by_key(|(window, set)| set.size()) {
                None => {
                    return callback((*self).clone());
                }
                Some((index, set)) => index,
            };
        for &word in self.sets[window].words() {
            let mut search2: Search = self.clone();
            search2.sets[window] = WordSet::from_words(&[word], word.len());
            for (window2, set2) in search2.sets.iter_mut() {
                if window2 != window {
                    set2.retain(|word2| word2 != word);
                }
            }
            search2.refine_one(window);
            search2.solve(callback)?;
        }
        Ok(())
    }

    pub fn letter_set_for_direction(&self, position: (usize, usize), direction: Direction) -> Option<LetterSet> {
        self.sets.window_at(position, direction).map(|window|
            self.sets[window].letters(window.offset(position).unwrap())
        )
    }

//    pub fn setletter_set_for_direction(&mut self, position: (usize, usize), direction: Direction) -> Option<&mut LetterSet> {
//        self.sets.window_at(position, direction).map(|window|
//            &mut self.sets[window].letters(window.offset(position))
//        )
//    }

    pub fn letter_set(&self, position: (usize, usize)) -> Option<LetterSet> {
        match (self.letter_set_for_direction(position, Direction::Across),
               self.letter_set_for_direction(position, Direction::Down)) {
            (None, x) => x,
            (x, None) => x,
            (Some(x), Some(y)) => Some(x.intersection(y))
        }
    }
    pub fn retain_letter_set(&mut self, position: (usize, usize), set: LetterSet) {
        for &direction in &[Direction::Across, Direction::Down] {
            if let Some(window) = self.sets.window_at(position, direction) {
                self.sets[window].retain(|word| {
                    set.get(word[window.offset(position).unwrap()])
                });
            }
        }
    }

//    pub fn set_letter_set(&mut self, position: (usize, usize), value: LetterSet) {
//        for &direction in &[Direction::Across, Direction::Down] {
//            if let Some(set) = self.letter_set_for_direction_mut(position, direction) {
//                *set = value;
//            }
//        }
//    }

    pub fn key(&self, positions: &Vec<(usize, usize)>) -> Vec<Letter> {
        positions.iter().map(|&position| self.letter_set(position).unwrap().unique().unwrap()).collect()
    }

    /*pub fn enumerate_cells(&self, cells: &[(usize, usize)], letters: &mut Vec<Letter>, callback: &mut dyn FnMut(&[Letter])) {
        if cells.len() == 0 {
            callback(letters);
        } else {
            for letter in self.letter_set(cells[0]).unwrap().into_iter() {
                //search2.set_letter_set(cells[0], [letter].iter().cloned().collect());
                //search2.enumerate_cells(&cells[1..], callback);
                unimplemented!();
            }
        }
    }*/

    pub fn solve_split(&self, overlap: &Vec<(usize, usize)>, children: &[Search; 2], callback: &mut dyn FnMut(Search) -> Result) -> Result {
        let mut overlap_values_iter =
            CartesianProduct::new(overlap.iter().map(|&position| self.letter_set(position).unwrap().into_iter()));
        while let Some(overlap_values) = overlap_values_iter.next() {
            let mut options: [Option<Search>; 2] = [None, None];
            for (index, child) in children.iter().enumerate() {
                let mut child_copy = child.clone();
                for (&position, &value) in overlap.iter().zip(overlap_values.iter()) {
                    child_copy.retain_letter_set(position, [value].iter().cloned().collect());
                }
                child_copy.refine_all();
                let _ = child_copy.solve(&mut take_one_result(&mut options[index]));
            }
            match options {
                [Some(r1), Some(r2)] => {
                    let mut combination = self.clone();
                    for (window, set) in combination.sets.iter_mut() {
                        for r in &[&r1, &r2] {
                            if let Some(result) = r.sets.get(window) {
                                *set = result.clone();
                            }
                        }
                        assert_eq!(set.size(), 1);
                    }
                    callback(combination)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn solve(&self, callback: &mut dyn FnMut(Search) -> Result) -> Result {
        if self.sets.values().any(|set| set.size() == 0) {
            return Ok(());
        }
        if Bernoulli::new(0.1).unwrap().sample(&mut rand::thread_rng()) {
            //println!("{:?}", self);
        }
        if self.sets.len() > 15 {
            if let Some((overlap, children)) = self.split_cells() {
                if children[0].sets.len() < self.sets.len() - 2 &&
                    children[1].sets.len() < self.sets.len() - 2 && overlap.len() <= 2 {
                    self.solve_split(&overlap, &children, callback)?;
                    return Ok(());
                }
            }
        }
        self.solve_direct(callback)?;
        Ok(())
    }
}