use std::collections::{HashSet, HashMap};
use std::iter::once;

use enum_map::Enum;
use enum_map::EnumMap;
use itertools::Itertools;

use crate::core::letter::Letter;
use crate::core::word::Word;
use crate::util::grid::Grid;
use std::ops::{Index, IndexMut};
use std::fmt::Display;
use std::fmt;
use crate::play::range_split::RangeSplitExt;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, Enum)]
pub enum Direction {
    Across,
    Down,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct Window {
    position: (u8, u8),
    length: u8,
    direction: Direction,
}

#[derive(Clone)]
pub struct WindowMap<T> {
    windows: HashMap<Window, T>,
    grid: Grid<EnumMap<Direction, Option<Window>>>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Cell {
    Black,
    White(Option<Letter>),
}

impl Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Cell::Black => write!(f, "█"),
            Cell::White(None) => write!(f, " "),
            Cell::White(Some(c)) => write!(f, "{}", c),
        }
    }
}

pub struct AsciiGrid<'a>(pub &'a Grid<Cell>);

impl<'a> Display for AsciiGrid<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for y in 0..self.0.size().1 {
            for x in 0..self.0.size().0 {
                write!(f, "{}", self.0[(x, y)])?;
            }
            writeln!(f)?;
        }
        writeln!(f)?;
        Ok(())
    }
}

impl Direction {
    pub fn perpendicular(self) -> Self {
        match self {
            Direction::Across => Direction::Down,
            Direction::Down => Direction::Across
        }
    }
}

impl Window {
    pub fn new(position: (usize, usize), length: usize, direction: Direction) -> Self {
        Window {
            position: (position.0 as u8, position.1 as u8),
            length: length as u8,
            direction: direction,
        }
    }
    pub fn position(&self) -> (usize, usize) {
        (self.position.0 as usize, self.position.1 as usize)
    }
    pub fn length(&self) -> usize {
        self.length as usize
    }
    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn position_at(&self, offset: usize) -> (usize, usize) {
        let (x0, y0) = self.position();
        match self.direction {
            Direction::Across => (x0 + offset, y0),
            Direction::Down => (x0, y0 + offset)
        }
    }
    pub fn positions<'a>(&'a self) -> impl Iterator<Item=(usize, usize)> + 'a {
        (0..self.length()).map(move |offset| self.position_at(offset))
    }
    pub fn offset(&self, position: (usize, usize)) -> usize {
        match self.direction {
            Direction::Across => {
                assert_eq!(self.position().1, position.1);
                assert!(self.position().0 <= position.0);
                assert!(position.0 < self.position().0 + self.length());
                position.0 - self.position().0
            }
            Direction::Down => {
                assert_eq!(self.position().0, position.0);
                assert!(self.position().1 <= position.1);
                assert!(position.1 < self.position().1 + self.length());
                position.1 - self.position().1
            }
        }
    }
}

impl<T> WindowMap<T> {
    pub fn new(windows: HashMap<Window, T>, size: (usize, usize)) -> Self {
        let mut grid = Grid::new(size, |_, _| EnumMap::new());
        for (window, value) in windows.iter() {
            for position in window.positions() {
                grid[position][window.direction] = Some(*window);
            }
        }

        WindowMap {
            windows,
            grid,
        }
    }

    pub fn len(&self) -> usize {
        self.windows.len()
    }
    pub fn grid_size(&self) -> (usize, usize) {
        self.grid.size()
    }
    pub fn windows<'a>(&'a self) -> impl Iterator<Item=Window> + 'a {
        self.windows.keys().cloned()
    }
    pub fn iter(&self) -> impl Iterator<Item=(Window, &T)> {
        self.windows.iter().map(|(&window, value)| (window, value))
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item=(Window, &mut T)> {
        self.windows.iter_mut().map(|(&window, value)| (window, value))
    }
    pub fn values(&self) -> impl Iterator<Item=&T> {
        self.windows.values()
    }
    pub fn window_at(&self, position: (usize, usize), direction: Direction) -> Option<Window> {
        self.grid[position][direction]
    }
    pub fn get(&self,window:Window)->Option<&T>{
        self.windows.get(&window)
    }
    /*pub fn verticals(&self) -> Vec<(usize, usize)> {
        iproduct!(0..self.grid.size().0,0..self.grid.size().1)
            .filter(|&p| {
                self.grid[p][Direction::Across].is_some()
                    || self.grid[p][Direction::Down].is_some()
            }).collect()
    }
    pub fn diagonals(&self) -> Vec<(usize, usize)> {
        let mut result = self.verticals();
        result.sort_by_key(|(x, y)| x + y);
        result
    }
    pub fn alternations(&self) -> Vec<usize> {
        self.windows.iter().enumerate().filter_map(
            |(i, x)| if x.direction() == Direction::Across { Some(i) } else { None }
        ).interleave(
            self.windows.iter().enumerate().filter_map(
                |(i, x)| if x.direction() == Direction::Down { Some(i) } else { None }
            )).collect()
    }*/
}


impl WindowMap<()> {
    pub fn from_grid(cells: &Grid<Cell>) -> Self {
        let mut windows = HashMap::new();
        for y in 0..cells.size().1 {
            for xs in (0..cells.size().0).range_split(|&x| cells[(x, y)] == Cell::Black) {
                let length = xs.end - xs.start;
                if length >= 2 {
                    windows.insert(Window::new((xs.start, y), length, Direction::Across), ());
                }
            }
        }
        for x in 0..cells.size().0 {
            for ys in (0..cells.size().1).range_split(|&y| cells[(x, y)] == Cell::Black) {
                let length = ys.end - ys.start;
                if length >= 2 {
                    windows.insert(Window::new((x, ys.start), length, Direction::Down), ());
                }
            }
        }
        WindowMap::new(windows, cells.size())
    }
}


impl<T> Index<Window> for WindowMap<T> {
    type Output = T;

    fn index(&self, index: Window) -> &Self::Output {
        &self.windows[&index]
    }
}

impl<T> IndexMut<Window> for WindowMap<T> {
    fn index_mut(&mut self, index: Window) -> &mut Self::Output {
        self.windows.get_mut(&index).unwrap()
    }
}

