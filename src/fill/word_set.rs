use std::collections::HashSet;
use std::fmt;
use std::fmt::{Error, Formatter};

use crate::core::letter::{ALPHABET, Letter, LetterSet};
use crate::core::word::Word;
use crate::util::bitset::IndexType;

#[derive(Clone)]
pub struct WordSet {
    words: Vec<Word>,
    table: Vec<[usize; ALPHABET]>,
}

impl WordSet {
    fn change_table(table: &mut Vec<[usize; ALPHABET]>, word: Word, delta: isize) {
        for i in 0..table.len() {
            let changed = &mut table[i][word[i].to_index()];
            *changed = ((*changed) as isize + delta) as usize;
        }
    }

    pub fn from_words(dict: &[Word], length: usize) -> Self {
        let mut table = vec![Default::default(); length];
        let mut words = vec![];
        for &word in dict.iter() {
            if word.len() == length {
                Self::change_table(&mut table, word, 1);
                words.push(word);
            }
        }

        WordSet {
            words: words,
            table: table,
        }
    }
    pub fn new(length: usize) -> Self {
        WordSet {
            words: vec![],
            table: vec![Default::default(); length],
        }
    }
    pub fn add_word(&mut self, word: Word) {
        self.words.push(word);
        Self::change_table(&mut self.table, word, 1);
    }
    pub fn retain<F: FnMut(Word) -> bool>(&mut self, mut predicate: F) {
        let table = &mut self.table;
        self.words.retain(|&word| {
            let retain = predicate(word);
            if !retain {
                Self::change_table(table, word, -1);
            }
            retain
        });
    }
    pub fn count(&self, index: usize, letter: Letter) -> usize {
        self.table[index][letter.to_index()]
    }
    pub fn letters(&self, index: usize) -> LetterSet {
        let mut result = LetterSet::from_value(false);
        for letter in Letter::all() {
            result.set(letter, self.table[index][letter.to_index()] > 0);
        }
        result
    }
    pub fn size(&self) -> usize {
        self.words.len()
    }
    pub fn words(&self) -> &[Word] {
        &self.words
    }

}

impl fmt::Debug for WordSet {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self.words)
    }
}


