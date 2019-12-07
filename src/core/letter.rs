use std::{fmt, io};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ops::{Index, IndexMut};

use arrayvec::ArrayVec;

use unidecode::unidecode;

use crate::util::bitset::{BitSet, IndexType};
use std::fmt::{Display, Formatter, Error};

pub const ALPHABET: usize = 26;

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Letter(u8);

pub type LetterSet = BitSet<Letter>;

impl IndexType for Letter {
    fn from_index(x: usize) -> Self {
        Letter(x as u8)
    }
    fn to_index(self) -> usize {
        self.0 as usize
    }
}

impl Letter {
    pub fn none() -> Self {
        Letter(255)
    }
    pub fn from_unicode(unicode: char) -> Option<Self> {
        let a = 'a' as u64;
        let A = 'A' as u64;
        let point = unicode as u64;
        let z = 'z' as u64;
        let Z = 'Z' as u64;
        if a <= point && point <= z {
            Some(Letter((point - a) as u8))
        } else if A <= point && point <= Z {
            Some(Letter((point - A) as u8))
        } else {
            None
        }
    }
    pub fn to_unicode(self) -> char {
        (self.0 + ('A' as u8)) as char
    }
    pub fn all() -> impl Iterator<Item=Self> {
        (0..ALPHABET).map(Self::from_index)
    }
}

impl Display for Letter {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.to_unicode())
    }
}