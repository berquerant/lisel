use crate::lineparse::Range;
use regex::Regex;

#[derive(Debug, Clone)]
pub enum Type {
    Re(Regex),
    Number(Range),
}

impl Type {
    pub fn select(&self, linum: u32, line: &str) -> bool {
        match &self {
            Type::Number(r) => match r {
                Range::Single(n) => *n == linum,
                Range::Interval(s, e) => *s <= linum && linum <= *e,
            },
            Type::Re(r) => r.is_match(line),
        }
    }
    pub fn start(&self) -> u32 {
        match &self {
            Type::Re(_) => std::u32::MIN,
            Type::Number(r) => match r {
                Range::Single(n) => *n,
                Range::Interval(s, _) => *s,
            },
        }
    }
    pub fn end(&self) -> u32 {
        match &self {
            Type::Re(_) => std::u32::MAX,
            Type::Number(r) => match r {
                Range::Single(n) => *n,
                Range::Interval(_, e) => *e,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    macro_rules! test_type_select {
        ($name:ident, $instance:expr, $linum:expr, $line:expr, $want:expr) => {
            #[test]
            fn $name() {
                let got = $instance.select($linum, $line);
                assert_eq!($want, got);
            }
        };
    }

    test_type_select!(
        type_select_re_matched,
        Type::Re(Regex::new("a").unwrap()),
        10,
        "a",
        true
    );
    test_type_select!(
        type_select_re_not_matched,
        Type::Re(Regex::new("a").unwrap()),
        10,
        "b",
        false
    );
    test_type_select!(
        type_select_number_single_matched,
        Type::Number(Range::Single(10)),
        10,
        "a",
        true
    );
    test_type_select!(
        type_select_number_single_not_matched,
        Type::Number(Range::Single(10)),
        11,
        "a",
        false
    );
    test_type_select!(
        type_select_number_interval_matched,
        Type::Number(Range::Interval(9, 11)),
        10,
        "a",
        true
    );
    test_type_select!(
        type_select_number_interval_not_matched,
        Type::Number(Range::Interval(10, 11)),
        12,
        "a",
        false
    );
    test_type_select!(
        type_select_number_interval_include_end_matched,
        Type::Number(Range::Interval(10, 10)),
        10,
        "a",
        true
    );
    test_type_select!(
        type_select_number_interval_without_size_not_matched,
        Type::Number(Range::Interval(11, 9)),
        10,
        "a",
        false
    );
}
