pub struct CartesianProduct<I: Iterator + Clone> {
    source: Vec<I>,
    state: Option<(Vec<I::Item>, Vec<I>)>,
}

impl<I: Iterator + Clone> CartesianProduct<I> {
    pub fn new<I2: IntoIterator<Item=I>>(iter: I2) -> Self {
        let source: Vec<I> = iter.into_iter().collect();
        CartesianProduct { source, state: None }
    }
    pub fn next(&mut self) -> Option<&[I::Item]> {
        if let Some((previous, next)) = &mut self.state {
            if !previous.iter_mut()
                .zip(next.iter_mut())
                .zip(self.source.iter())
                .rev()
                .any(|((p, n), s)| {
                if let Some(x) = n.next() {
                    *p = x;
                    true
                } else {
                    *n = s.clone();
                    *p = n.next().unwrap();
                    false
                }
            }) {
                self.state = None
            }
        } else {
            let mut next = self.source.clone();
            if let Some(previous) = next.iter_mut().map(|it| it.next()).collect() {
                self.state = Some((previous, next));
            }
        }
        self.state.as_ref().map(|x| x.0.as_slice())
    }
}

fn cartesian_product_impl<'a, T>(source: &[&'a [T]], sink: &mut Vec<&'a T>, callback: &mut dyn FnMut(&[&T])) {
    if source.len() == 0 {
        callback(sink);
    } else {
        for x in source[0] {
            sink.push(x);
            cartesian_product_impl(&source[1..], sink, callback);
            sink.pop();
        }
    }
}

pub fn cartesian_product<T>(source: &[&[T]], callback: &mut dyn FnMut(&[&T])) {
    cartesian_product_impl(source, &mut vec![], callback);
}

#[test]
fn test_cartesian_product() {
    fn test(source: &[&[usize]], expected: &[&[usize]]) {
        let mut actual: Vec<Vec<usize>> = vec![];
        cartesian_product(source, &mut |result: &[&usize]| {
            actual.push(result.iter().cloned().cloned().collect());
        });
        let mut actual2: Vec<Vec<usize>> = vec![];
        let mut product = CartesianProduct::new(source.iter().map(|x| x.iter()));
        while let Some(value) = product.next() {
            actual2.push(value.iter().cloned().cloned().collect());
        }
        assert!(expected.iter().cloned().eq(actual.iter().map(|x| x.as_slice())), "{:?} != {:?}", expected, actual);
        assert!(expected.iter().cloned().eq(actual2.iter().map(|x| x.as_slice())), "{:?} != {:?}", expected, actual2);
    }
    test(&[], &[&[]]);
    test(&[&[]], &[]);
    test(&[&[1]], &[&[1]]);
    test(&[&[1, 2]], &[&[1], &[2]]);
    test(&[&[1], &[2]], &[&[1, 2]]);
    test(&[&[1, 2], &[3]], &[&[1, 3], &[2, 3]]);
    test(&[&[1], &[2, 3]], &[&[1, 2], &[1, 3]]);
    test(&[&[1, 2], &[3, 4]], &[&[1, 3], &[1, 4], &[2, 3], &[2, 4]]);
}
