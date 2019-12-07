use std::marker::PhantomData;
use std::mem::size_of;
use std::ops::{Index, BitOrAssign};
use std::fmt;
use std::iter::FromIterator;

pub trait IndexType {
    fn from_index(x: usize) -> Self;
    fn to_index(self) -> usize;
}

impl IndexType for usize {
    fn from_index(x: usize) -> Self {
        x
    }
    fn to_index(self) -> usize {
        self
    }
}

#[derive(Eq, Ord, PartialEq, PartialOrd, Hash, Copy)]
pub struct BitSet<T: IndexType> {
    buffer: usize,
    phantom: PhantomData<T>,
}

impl<T: IndexType> Clone for BitSet<T> {
    fn clone(&self) -> Self {
        BitSet {
            buffer: self.buffer,
            phantom: PhantomData,
        }
    }
}

impl<T: IndexType + 'static> BitSet<T> {
    fn new(buffer: usize) -> Self {
        BitSet { buffer, phantom: PhantomData }
    }
    pub fn from_value(value: bool) -> Self {
        BitSet { buffer: if value { !0 } else { 0 }, phantom: PhantomData }
    }
    pub fn get(&self, index: T) -> bool {
        (self.buffer >> index.to_index()) & 1 == 1
    }
    pub fn set(&mut self, index: T, value: bool) {
        if value {
            self.buffer |= 1 << index.to_index();
        } else {
            self.buffer &= !(1 << index.to_index());
        }
    }
    pub fn unique(self) -> Option<T> {
        if self.buffer == 0 {
            None
        } else if self.buffer & (self.buffer - 1) != 0 {
            None
        } else {
            Some(T::from_index(self.buffer.trailing_zeros() as usize))
        }
    }
    pub fn is_subset(self, other: Self) -> bool {
        (self.buffer | other.buffer) == other.buffer
    }
    pub fn len() -> usize {
        size_of::<usize>() * 8
    }
    pub fn intersection(self, other: Self) -> Self {
        Self::new(self.buffer & other.buffer)
    }
    pub fn into_iter(self) -> impl Iterator<Item=T> + Clone {
        (0..Self::len()).filter(move |&x| self.get(T::from_index(x))).map(T::from_index)
    }
}

#[test]
fn test_bitset() {
    let mut bitset = BitSet::from_value(false);
    assert_eq!(bitset.unique(), None);
    for i in 0..BitSet::<usize>::len() {
        assert_eq!(bitset.get(i), false);
    }
    bitset.set(3, true);
    assert_eq!(bitset.unique(), Some(3));
    for i in 0..BitSet::<usize>::len() {
        assert_eq!(bitset.get(i), i == 3);
    }
    bitset.set(63, true);
    assert_eq!(bitset.unique(), None);
    for i in 0..BitSet::<usize>::len() {
        assert_eq!(bitset.get(i), i == 3 || i == 63);
    }
}

impl<T: IndexType> BitOrAssign for BitSet<T> {
    fn bitor_assign(&mut self, rhs: Self) {
        self.buffer |= rhs.buffer
    }
}

impl<T: IndexType + fmt::Debug + 'static> fmt::Debug for BitSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut set = f.debug_set();
        for x in 0..Self::len() {
            if self.get(T::from_index(x)) {
                set.entry(&T::from_index(x));
            }
        }
        set.finish()
    }
}

impl<T: IndexType + 'static> FromIterator<T> for BitSet<T> {
    fn from_iter<I: IntoIterator<Item=T>>(iter: I) -> Self {
        let mut result = BitSet::from_value(false);
        for x in iter {
            result.set(x, true);
        }
        result
    }
}