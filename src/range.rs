use nom::{
    branch::alt,
    character::complete::{char, i32},
    combinator::map,
    sequence::{preceded, separated_pair, terminated},
    IResult,
};

use crate::parse::ParseError;

#[derive(Debug, PartialEq, Clone)]
pub struct Range(Option<i32>, Option<i32>);

impl Range {
    pub fn new(bounds: (i32, i32)) -> Self {
        Range(Some(bounds.0), Some(bounds.1))
    }

    pub fn lower(i: i32) -> Self {
        Range(Some(i), None)
    }

    pub fn upper(i: i32) -> Self {
        Range(None, Some(i))
    }

    pub fn normalize(&self, len: usize) -> std::ops::Range<usize> {
        let normalize_bound = |bound: i32| {
            if bound < 0 {
                let u = -bound as usize;
                if u > len {
                    0
                } else {
                    len - u
                }
            } else {
                let u = bound as usize;
                if u > len {
                    len
                } else {
                    u
                }
            }
        };

        match (self.0.map(normalize_bound), self.1.map(normalize_bound)) {
            (None, None) => unreachable!(),
            (None, Some(u)) => 0..u,
            (Some(l), None) => l..len,
            (Some(l), Some(u)) => l..u,
        }
    }
}

pub(crate) fn parse(input: &str) -> IResult<&str, Range, ParseError> {
    alt((
        map(separated_pair(i32, char(':'), i32), Range::new),
        map(preceded(char(':'), i32), Range::upper),
        map(terminated(i32, char(':')), Range::lower),
    ))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_full() {
        assert_eq!(1..3, Range::new((1, 3)).normalize(10));
        assert_eq!(1..3, Range::new((1, 10)).normalize(3));
        assert_eq!(0..3, Range::new((-100, 3)).normalize(10));
        assert_eq!(1..8, Range::new((1, -2)).normalize(10));
        assert_eq!(0..10, Range::new((-100, 100)).normalize(10));
        assert_eq!(3..2, Range::new((3, 2)).normalize(10));
        assert_eq!(7..8, Range::new((-3, -2)).normalize(10));
    }

    #[test]
    fn normalize_lower() {
        assert_eq!(1..10, Range::lower(1).normalize(10));
        assert_eq!(9..10, Range::lower(-1).normalize(10));
        assert_eq!(10..10, Range::lower(100).normalize(10));
        assert_eq!(0..10, Range::lower(-100).normalize(10));
    }

    #[test]
    fn normalize_upper() {
        assert_eq!(0..1, Range::upper(1).normalize(10));
        assert_eq!(0..9, Range::upper(-1).normalize(10));
        assert_eq!(0..10, Range::upper(100).normalize(10));
        assert_eq!(0..0, Range::upper(-100).normalize(10));
    }
}
