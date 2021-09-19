use crate::{
    combinator::{chain, optional, Chain, Split},
    construction::Construct,
    index::Index,
    operators::{Op, Sign},
    query::Query,
    raw::Raw,
    space,
};
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alphanumeric1, char},
    combinator::{all_consuming, map, opt, value},
    error::{self, ErrorKind},
    sequence::{pair, preceded},
    IResult,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Incomplete: {0}")]
    Incomplete(String),
    #[error("Invalid format: {0:?} at {1}")]
    InvalidFormat(ErrorKind, String),
}

impl From<nom::Err<ParseError>> for ParseError {
    fn from(err: nom::Err<ParseError>) -> Self {
        match err {
            nom::Err::Incomplete(n) => ParseError::Incomplete(format!("{:?}", n)),
            nom::Err::Error(e) | nom::Err::Failure(e) => e,
        }
    }
}

impl error::ParseError<&str> for ParseError {
    fn from_error_kind(input: &str, kind: ErrorKind) -> Self {
        ParseError::InvalidFormat(kind, input.to_string())
    }

    fn append(_: &str, _: ErrorKind, other: Self) -> Self {
        other
    }
}

impl std::str::FromStr for Query {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Query::parse(s)
    }
}

pub trait Parseable: Sized {
    fn parser(input: &str) -> IResult<&str, Self, ParseError>;

    fn parse(input: &str) -> Result<Self, ParseError> {
        let (_, output) = all_consuming(Self::parser)(input)?;
        Ok(output)
    }
}

impl Parseable for Query {
    fn parser(input: &str) -> IResult<&str, Self, ParseError> {
        if input.is_empty() {
            return Ok((input, Query::Empty));
        }
        parse_pipe(input)
    }
}

pub(crate) fn parse_pipe(input: &str) -> IResult<&str, Query, ParseError> {
    let (input, curr) = parse_split(input)?;
    let (input, opt) = opt(preceded(space::around(char('|')), parse_pipe))(input)?;
    if let Some(next) = opt {
        Ok((input, Query::Chain(Box::new(Chain(curr, next)))))
    } else {
        Ok((input, curr))
    }
}

pub(crate) fn parse_split(input: &str) -> IResult<&str, Query, ParseError> {
    let (input, left) = parse_op(input)?;
    let (input, opt) = opt(preceded(space::around(char(',')), parse_split))(input)?;
    if let Some(right) = opt {
        Ok((input, Query::Split(Box::new(Split(left, right)))))
    } else {
        Ok((input, left))
    }
}

pub(crate) fn parse_op(input: &str) -> IResult<&str, Query, ParseError> {
    let (input, left) = parse_init(input)?;
    let (input, opt) = opt(pair(space::around(Sign::parser), parse_op))(input)?;
    if let Some((sign, right)) = opt {
        Ok((input, Query::Op(Box::new(Op { left, sign, right }))))
    } else {
        Ok((input, left))
    }
}

pub(crate) fn parse_init(input: &str) -> IResult<&str, Query, ParseError> {
    space::around(alt((
        chain(alt((
            parse_index_shorthand,
            map(Construct::parser, Query::Contruct),
            preceded(char('.'), alt((parse_index, parse_iterator))),
        ))),
        map(Raw::parser, Query::Raw),
        value(Query::Recurse, tag("..")),
        value(Query::Identity, char('.')),
    )))(input)
}

pub(crate) fn parse_chain(input: &str) -> IResult<&str, Query, ParseError> {
    chain(alt((parse_index_shorthand, parse_index, parse_iterator)))(input)
}

fn parse_index(input: &str) -> IResult<&str, Query, ParseError> {
    optional(map(Index::parser, Query::Index))(input)
}

fn parse_index_shorthand(input: &str) -> IResult<&str, Query, ParseError> {
    optional(map(preceded(char('.'), alphanumeric1), |s: &str| {
        Query::Index(Index::String(s.to_string()))
    }))(input)
}

fn parse_iterator(input: &str) -> IResult<&str, Query, ParseError> {
    optional(value(Query::Iterator, tag("[]")))(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        assert!("...".parse::<Query>().is_err());
        assert_eq!(Query::Recurse, "..".parse().unwrap());
        assert_eq!(Query::Identity, ".".parse().unwrap());
        assert_eq!(Query::Empty, "".parse().unwrap());
    }

    #[test]
    fn iterator() {
        assert!("[]".parse::<Query>().is_err());
        assert!(".[".parse::<Query>().is_err());
        assert!(".]".parse::<Query>().is_err());
        assert!(".[].[]".parse::<Query>().is_err());

        assert_eq!(Query::Iterator, ".[]".parse().unwrap());
    }
}
