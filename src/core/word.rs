use std::{fmt, io};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ops::{Index, IndexMut, Deref};

use arrayvec::ArrayVec;

use unidecode::unidecode;

use crate::core::letter::Letter;
use crate::util::bitset::{BitSet, IndexType};
use std::iter::FromIterator;

pub const WIDTH: usize = 16;

#[derive(Eq, Ord, PartialEq, PartialOrd, Hash, Copy, Clone)]
pub struct Word {
    length: usize,
    buffer: [Letter; WIDTH],
}

impl Word {
    pub fn new() -> Self {
        Word {
            length: 0,
            buffer: [Letter::none(); WIDTH],
        }
    }
    pub fn from_str(input: &str) -> Option<Self> {
        let mut buffer = [Letter::none(); WIDTH];
        let mut length = 0;
        for c in unidecode(input).chars() {
            if let Some(l) = Letter::from_unicode(c) {
                if length == WIDTH {
                    return None;
                }
                buffer[length] = l;
                length += 1;
            }
        }
        Some(Word {
            length,
            buffer,
        })
    }
    pub fn push(&mut self, letter: Letter) {
        if self.buffer.len() == self.length {
            panic!("capacity is full");
        }
        self.buffer[self.length] = letter;
        self.length += 1;
    }
    pub fn len(&self) -> usize {
        self.length
    }
    pub fn from_lines(file: &str) -> io::Result<Vec<Word>> {
        let f = File::open(file)?;
        let f = BufReader::new(f);
        let mut result = vec![];
        for line in f.lines() {
            let str = line?;
            if let Some(word) = Word::from_str(&str) {
                result.push(word);
            }
        }
        Ok(result)
    }
}

impl Index<usize> for Word {
    type Output = Letter;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.length, "{:?} < {:?}", index, self.length);
        &self.buffer[index]
    }
}

impl IndexMut<usize> for Word {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.length, "{:?} < {:?}", index, self.length);
        &mut self.buffer[index]
    }
}

impl fmt::Debug for Letter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_unicode())
    }
}

impl fmt::Debug for Word {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for i in 0..self.length {
            write!(f, "{:?}", self.buffer[i])?;
        }
        Ok(())
    }
}

impl FromIterator<Letter> for Word {
    fn from_iter<T: IntoIterator<Item=Letter>>(iter: T) -> Self {
        let mut result = Word::new();
        for letter in iter {
            result.push(letter);
        }
        result
    }
}

impl Deref for Word {
    type Target = [Letter];

    fn deref(&self) -> &Self::Target {
        &self.buffer[0..self.length]
    }
}
