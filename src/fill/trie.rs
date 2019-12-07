use crate::core::letter::{ALPHABET, LetterSet, Letter};
use crate::core::word::Word;
use slice_group_by::GroupBy;
use crate::util::bitset::IndexType;
use std::iter::FromIterator;
use std::collections::HashMap;

lazy_static! {
  static ref EMPTY_TRIE:Trie = Trie::empty();
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trie {
    length: usize,
    suffixes: Vec<LetterSet>,
    children: HashMap<Letter, Trie>,
}

impl Trie {
    pub fn empty() -> Trie {
        Trie {
            length: 0,
            suffixes: Vec::new(),
            children: HashMap::new(),
        }
    }

    pub fn child(&self, letter: Letter) -> &Trie {
        self.children.get(&letter).unwrap_or(&EMPTY_TRIE)
    }

    pub fn suffixes(&self) -> &[LetterSet] {
        &self.suffixes
    }

    pub fn len(&self) -> usize {
        self.length
    }

    fn from_words_impl(words: &[Word], index: usize) -> Trie {
        if words.len() == 0 {
            return Self::empty();
        }
        let children_words = if words[0].len() == index {
            &words[1..]
        } else {
            words
        };
        let children: HashMap<Letter, Trie> = children_words.binary_group_by_key(|word| word[index]).map(|group| {
            (group[0][index], Trie::from_words_impl(group, index + 1))
        }).collect();
        let mut suffixes: Vec<LetterSet> = vec![];
        for (&letter, child) in children.iter() {
            suffixes.resize(suffixes.len().max(child.suffixes().len() + 1), LetterSet::from_value(false));
            suffixes[0].set(letter, true);
            for (i, s) in child.suffixes().iter().enumerate() {
                suffixes[i + 1] |= *s;
            }
        }
        Trie { length: words.len(), suffixes, children }
    }
}

impl FromIterator<Word> for Trie {
    fn from_iter<T: IntoIterator<Item=Word>>(iter: T) -> Self {
        let mut words: Vec<Word> = iter.into_iter().collect();
        words.sort();
        words.dedup();
        Self::from_words_impl(&words, 0)
    }
}

#[test]
fn test_trie() {
    fn suffixes(sets: &[&[char]]) -> Vec<LetterSet> {
        sets.iter().map(
            |set| set.iter().map(
                |&c| Letter::from_unicode(c).unwrap()).collect()).collect()
    }
    let trie: Trie = ["ab", "ac"].iter().map(|w| Word::from_str(w).unwrap()).collect();
    let trieA=trie.child(Letter::from_unicode('a').unwrap());
    let trieB=trie.child(Letter::from_unicode('b').unwrap());
    let trieAB=trieA.child(Letter::from_unicode('b').unwrap());
    let trieAC=trieA.child(Letter::from_unicode('c').unwrap());
    let trieABC=trieAB.child(Letter::from_unicode('c').unwrap());

    assert_eq!(suffixes(&[&['a'], &['b', 'c']]).as_slice(), trie.suffixes());
    assert_eq!(trie.len(), 2);
    assert_eq!(suffixes(&[&['b', 'c']]).as_slice(), trieA.suffixes());
    assert_eq!(trieA.len(), 2);
    assert_eq!(suffixes(&[]).as_slice(), trieB.suffixes());
    assert_eq!(trieB.len(), 0);
    assert_eq!(suffixes(&[]).as_slice(), trieAB.suffixes());
    assert_eq!(trieAB.len(), 1);
    assert_eq!(suffixes(&[]).as_slice(), trieAC.suffixes());
    assert_eq!(trieAC.len(), 1);
    assert_eq!(suffixes(&[]).as_slice(), trieABC.suffixes());
    assert_eq!(trieABC.len(), 0);

}