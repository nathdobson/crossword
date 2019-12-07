use std::ops::{Index, IndexMut, Range};
use crate::core::puzzle::Direction;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Grid<T> {
    size: (usize, usize),
    elements: Vec<T>,
}

impl<T> Grid<T> {
    pub fn new<F: FnMut(usize, usize) -> T>(size: (usize, usize), mut builder: F) -> Grid<T> {
        let mut elements = Vec::with_capacity(size.0 * size.1);
        for y in 0..size.1 {
            for x in 0..size.0 {
                elements.push(builder(x, y));
            }
        }
        Grid { size, elements }
    }
    pub fn size(&self) -> (usize, usize) {
        self.size
    }
    pub fn iter(&self) -> impl Iterator<Item=&T> {
        self.elements.iter()
    }
}

impl<T> Index<(usize, usize)> for Grid<T> {
    type Output = T;

    fn index(&self, index: (usize, usize)) -> &Self::Output {
        assert!(index.0 < self.size.0);
        assert!(index.1 < self.size.1);
        &self.elements[index.0 + index.1 * self.size.0]
    }
}

impl<T> IndexMut<(usize, usize)> for Grid<T> {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        assert!(index.0 < self.size.0);
        assert!(index.1 < self.size.1);
        &mut self.elements[index.0 + index.1 * self.size.0]
    }
}