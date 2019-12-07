use std::ops::Range;
use std::iter::Step;
use std::collections::HashMap;

pub struct RangeSplit<T: Step + Clone, F> {
    range: Option<Range<T>>,
    pred: F,
}

pub trait RangeSplitExt<T: Step + Clone, F: FnMut(&T) -> bool>: Sized {
    type RangeSplit: Iterator<Item=Self>;
    fn range_split(self, pred: F) -> Self::RangeSplit;
}

impl<T: Step + Clone, F: FnMut(&T) -> bool> RangeSplitExt<T, F> for Range<T> {
    type RangeSplit = RangeSplit<T, F>;
    fn range_split(self, pred: F) -> Self::RangeSplit {
        RangeSplit {
            range: Some(self),
            pred: pred,
        }
    }
}

impl<T: Step + Clone, F: FnMut(&T) -> bool> Iterator for RangeSplit<T, F> {
    type Item = Range<T>;
    fn next(&mut self) -> Option<Range<T>> {
        match self.range.take() {
            None => None,
            Some(range) => {
                let mut remainder = range.clone();
                match remainder.position(|x| (self.pred)(&x)) {
                    None => Some(range),
                    Some(position) => {
                        let end = range.start.add_usize(position).unwrap();
                        let result = range.start..end;
                        self.range = Some(remainder);
                        return Some(result);
                    }
                }
            }
        }
    }
}

#[test]
fn test_range_split() {
    fn test(expected: Vec<Range<usize>>, range: Range<usize>, values: Vec<bool>) {
        let mut table = HashMap::<usize, bool>::new();
        for (i, x) in values.into_iter().enumerate() {
            table.insert(i, x);
        }
        assert_eq!(expected, range.range_split(|&x| table.remove(&x).unwrap()).collect::<Vec<Range<usize>>>());
    }
    test(vec![0..0], 0..0, vec![]);
    test(vec![0..1], 0..1, vec![false]);
    test(vec![0..0, 1..1], 0..1, vec![true]);
    test(vec![0..2], 0..2, vec![false, false]);
    test(vec![0..1, 2..2], 0..2, vec![false, true]);
    test(vec![0..0, 1..2], 0..2, vec![true, false]);
    test(vec![0..0, 1..1, 2..2], 0..2, vec![true, true]);
}