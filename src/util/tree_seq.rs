use std::cell::Cell;
use std::fmt;
use std::rc::Rc;
use std::sync::Weak;

pub enum TreeSeq<T> {
    Node {
        left: Box<TreeSeq<T>>,
        right: Box<TreeSeq<T>>,
    },
    Leaf(T),
}

impl<T> TreeSeq<T> {
    pub fn leaf(x: T) -> Self {
        Self::Leaf(x)
    }
    pub fn concat(self, other: Self) -> Self {
        TreeSeq::Node {
            left: box self,
            right: box other,
        }
    }
    pub fn iter(&self) -> Iter<T> {
        Iter {
            stack: vec![self]
        }
    }
}

pub struct Iter<'a, T: 'a> {
    stack: Vec<&'a TreeSeq<T>>,
}

impl<'a, T: 'a> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(next) = self.stack.pop() {
            match next {
                TreeSeq::Node { left, right } => {
                    self.stack.push(right);
                    self.stack.push(left);
                }
                TreeSeq::Leaf(value) => return Some(value)
            }
        }
        None
    }
}

impl<T: fmt::Debug> fmt::Debug for TreeSeq<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[test]
fn test_iter() {
    fn leaf(x: usize) -> TreeSeq<usize> {
        TreeSeq::leaf(x)
    }
    fn flatten(x: TreeSeq<usize>) -> Vec<usize> {
        x.iter().cloned().collect()
    }
    assert_eq!(flatten(leaf(0)), vec![0]);
    assert_eq!(flatten(leaf(0).concat(leaf(1))), vec![0, 1]);
    assert_eq!(flatten((leaf(0).concat(leaf(1))).concat(leaf(2))), vec![0, 1, 2]);
    assert_eq!(flatten(leaf(0).concat(leaf(1).concat(leaf(2)))), vec![0, 1, 2]);
    assert_eq!(flatten((leaf(0).concat(leaf(1))).concat(leaf(2).concat(leaf(3)))), vec![0, 1, 2, 3]);
}