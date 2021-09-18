use nom::{
    IResult, 
    branch::alt, 
    bytes::complete::take_while1, 
    character::complete::{char, i32}, 
    combinator::map, 
    number::complete::float, 
    sequence::delimited
};
use serde_json::{Number, Value};

use crate::{QueryResult, parse::ParseError, single};

#[derive(Debug, PartialEq, Clone)]
pub enum Raw {
    String(String),
    Number(Number),
}

impl Raw {
    pub fn execute(&self) -> QueryResult {
        let v = match self {
            Raw::String(s) => Value::String(s.clone()),
            Raw::Number(n) => Value::Number(n.clone()),
        };
        single(v)
    }
}

pub(crate) fn parse(input: &str) -> IResult<&str, Raw, ParseError> {
    alt((
        map(delimited(
            char('"'), 
            take_while1(|c| c != '"'), 
            char('"')
        ), |s: &str| Raw::String(s.to_string())),
        map(i32, |n| Raw::Number(Number::from(n))),
        map(float, |n| Raw::Number(Number::from_f64(n as f64).unwrap())),
    ))(input)
}