use std::collections::HashMap;
use std::collections::hash_map::Values;

pub struct BagToken(usize);

pub struct Bag<T> {
    map: HashMap<usize, T>,
    next: usize,
}

impl<T> Bag<T> {
    pub fn new() -> Self {
        Bag {
            map: HashMap::new(),
            next: 0,
        }
    }
    pub fn insert(&mut self, value: T) -> BagToken {
        let key = self.next;
        self.next += 1;
        self.map.insert(key, value);
        BagToken(key)
    }
    pub fn remove(&mut self, token: BagToken) -> T {
        self.map.remove(&token.0).unwrap()
    }
}

impl<'a, T> IntoIterator for &'a Bag<T> {
    type Item = &'a T;
    type IntoIter = Values<'a, usize, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.map.values()
    }
}

#[test]
fn test_bag() {
    let mut bag = Bag::<usize>::new();
    let t1 = bag.insert(1);
    let t2 = bag.insert(2);
    bag.remove(t1);
    assert_eq!(vec![2], bag.into_iter().map(|x| *x).collect::<Vec<usize>>());
}