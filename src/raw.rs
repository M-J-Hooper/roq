use nom::{
    branch::alt,
    bytes::complete::take_while,
    character::complete::{char, i32},
    combinator::map,
    number::complete::float,
    sequence::delimited,
    IResult,
};
use serde_json::{Number, Value};

use crate::{
    parse::{ParseError, Parseable},
    query::Executable,
    single, QueryResult,
};

#[derive(Debug, PartialEq, Clone)]
pub struct Raw(Value);

impl Executable for Raw {
    fn execute(&self, _: &Value) -> QueryResult {
        single(&self.0)
    }
}

impl Parseable for Raw {
    fn parser(input: &str) -> IResult<&str, Self, ParseError> {
        map(
            alt((
                map(
                    delimited(char('"'), take_while(|c| c != '"'), char('"')),
                    |s: &str| Value::String(s.to_string()),
                ),
                map(i32, |n| Value::Number(Number::from(n))),
                map(float, |n| {
                    Value::Number(Number::from_f64(n as f64).unwrap())
                }),
            )),
            Raw,
        )(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_raw_string() {
        assert!(Raw::parse("foo").is_err());
        assert!(Raw::parse("\"foo").is_err());
        assert!(Raw::parse("foo\"").is_err());

        assert_eq!(
            Raw(Value::String("".to_string())),
            Raw::parse("\"\"").unwrap()
        );
        assert_eq!(
            Raw(Value::String("f o o".to_string())),
            Raw::parse("\"f o o\"").unwrap()
        );
    }

    #[test]
    fn parse_raw_number() {
        assert!(Raw::parse("--4").is_err());
        assert!(Raw::parse("0..5").is_err());
        assert!(Raw::parse("4 4").is_err());

        assert_eq!(
            Raw(Value::Number(Number::from(-4))),
            Raw::parse("-4").unwrap()
        );
        assert_eq!(
            Raw(Value::Number(Number::from_f64(0.5).unwrap())),
            Raw::parse(".5").unwrap()
        );
    }
}
