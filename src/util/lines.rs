use xi_unicode::LineBreakIterator;
use unicode_segmentation::UnicodeSegmentation;

pub fn break_lines(string: &str, max_graphemes: usize) -> impl Iterator<Item=&str> {
    let mut weights = Vec::with_capacity(string.len());
    {
        let mut weight = 0;
        for grapheme in string.graphemes(true) {
            for _ in 0..grapheme.len() {
                weights.push(weight);
            }
            weight += 1;
        }
        weights.push(weight);
    }
    let mut result = vec![];
    {
        let mut base = 0;
        let mut breaks = LineBreakIterator::new(string).peekable();
        loop {
            let mut index: usize = base;
            while let Some(&(next_index, forced)) = breaks.peek() {
                if next_index <= base {
                    breaks.next();
                } else if weights[next_index] - weights[base] <= max_graphemes {
                    index = next_index;
                    breaks.next();
                    if forced { break; }
                } else {
                    break;
                }
            }
            if index == base {
                index = base + weights[base..].iter().position(
                    |&weight| weight == weights[base] + max_graphemes).unwrap_or(string.len() - base);
            }
            if index == base {
                assert_eq!(index, string.len());
                break;
            } else {
                result.push(&string[base..index]);
                base = index;
            }
        }
    }
    result.into_iter()
}

#[test]
fn test_break_lines() {
    fn test(input: &str, max_graphemes: usize, output: &[&str]) {
        assert_eq!(break_lines(input, max_graphemes).collect::<Vec<_>>(), output.to_vec());
    }
    test("", 1, &[]);
    test("x", 1, &["x"]);
    test("  ", 1, &[" ", " "]);
    test("x ", 1, &["x", " "]);
    test(" y", 1, &[" ", "y"]);
    test("xy", 1, &["x", "y"]);
    test("xy", 2, &["xy"]);
    test("a bc", 2, &["a ", "bc"]);
    test("ab c", 2, &["ab", " c"]);
    test("a bc", 3, &["a ", "bc"]);
    test("ab c", 3, &["ab ", "c"]);
    test("abcdef 123456", 10, &["abcdef ", "123456"]);
    test("abcdef 123456 uvwxyz", 10, &["abcdef ", "123456 ", "uvwxyz"]);
    test("abcdef123456 uvwxyz", 10, &["abcdef1234", "56 uvwxyz"]);
    test("abcd 12345", 10, &["abcd 12345"]);
}