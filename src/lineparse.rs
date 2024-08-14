use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::one_of,
    combinator::{fail, recognize},
    multi::many1,
    sequence::{preceded, separated_pair, terminated},
    IResult,
};
use std::clone::Clone;
use std::cmp::PartialEq;

/// Expressions arranged in rows of index file.
#[derive(Debug, PartialEq, Clone)]
pub enum Range {
    /// NATURAL_NUMBER
    Single(u32),
    /// NATURAL_NUMBER,NATURAL_NUMBER
    /// ,NATURAL_NUMBER
    /// NATURAL_NUMBER,
    Interval(u32, u32),
}

/// Parse a natural number.
fn natural(input: &str) -> IResult<&str, u32> {
    let (input, value) = recognize(many1(one_of("0123456789")))(input)?;
    let v: u32 = value.parse().unwrap();
    if v < 1 {
        fail(input)
    } else {
        Ok((input, v))
    }
}

fn single(input: &str) -> IResult<&str, Range> {
    let (input, value) = natural(input)?;
    Ok((input, Range::Single(value)))
}

fn interval_left_open(input: &str) -> IResult<&str, Range> {
    let (input, value) = preceded(tag(","), natural)(input)?;
    Ok((input, Range::Interval(u32::MIN, value)))
}

fn interval_right_open(input: &str) -> IResult<&str, Range> {
    let (input, value) = terminated(natural, tag(","))(input)?;
    Ok((input, Range::Interval(value, u32::MAX)))
}

fn interval(input: &str) -> IResult<&str, Range> {
    let (input, (left_limit, right_limit)) = separated_pair(natural, tag(","), natural)(input)?;
    Ok((input, Range::Interval(left_limit, right_limit)))
}

pub fn range(input: &str) -> IResult<&str, Range> {
    alt((interval, interval_left_open, interval_right_open, single))(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    macro_rules! test_range {
        ($name:ident, $input:expr, $want:expr) => {
            #[test]
            fn $name() {
                let got = range($input);
                assert_eq!($want, got);
            }
        };
    }

    macro_rules! test_range_error {
        ($name:ident, $input:expr) => {
            #[test]
            fn $name() {
                let got = range($input);
                assert!(got.is_err());
            }
        };
    }

    test_range!(parse_single, "4", Ok(("", Range::Single(4))));
    test_range!(parse_interval, "4,8", Ok(("", Range::Interval(4, 8))));
    test_range!(
        parse_interval_identical,
        "1,1",
        Ok(("", Range::Interval(1, 1)))
    );
    test_range!(
        parse_interval_left_open,
        ",5",
        Ok(("", Range::Interval(std::u32::MIN, 5)))
    );
    test_range!(
        parse_interval_right_open,
        "5,",
        Ok(("", Range::Interval(5, std::u32::MAX)))
    );
    test_range!(parse_interval_empty, "4,3", Ok(("", Range::Interval(4, 3))));
    test_range_error!(parse_single_error_not_narural, "0");
    test_range_error!(parse_interval_error_not_natural, "-1,2");
}
