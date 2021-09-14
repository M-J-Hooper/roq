use crate::{
    parse::ParseError,
    range::{self, Range},
};
use nom::{
    branch::alt,
    bytes::complete::take_while1,
    character::complete::{char, i32},
    combinator::map,
    sequence::delimited,
    IResult,
};

#[derive(Debug, PartialEq, Clone)]
pub enum Index {
    String(String),
    Integer(i32),
    Slice(Range),
}

pub(crate) fn parse(input: &str) -> IResult<&str, Index, ParseError> {
    delimited(
        char('['),
        alt((
            map(range::parse, Index::Slice),
            map(i32, Index::Integer),
            map(
                delimited(char('"'), take_while1(|c| c != '"'), char('"')),
                |s: &str| Index::String(s.to_string()),
            ),
        )),
        char(']'),
    )(input)
}
