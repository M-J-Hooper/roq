use crate::combinator::{chain, optional, Chain, Split};
use crate::construction::Construct;
use crate::index::Index;
use crate::query::Query;
use crate::space;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alphanumeric1, char},
    combinator::{all_consuming, map, opt, value},
    error::{self, ErrorKind},
    sequence::preceded,
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

pub trait Parseable: Sized {
    fn parse(input: &str) -> IResult<&str, Self, ParseError>;
}

impl Parseable for Query {
    fn parse(input: &str) -> IResult<&str, Self, ParseError> {
        if input.is_empty() {
            return Ok((input, Query::Empty));
        }
        parse_pipe(input)
    }
}

impl std::str::FromStr for Query {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (_, query) = all_consuming(Query::parse)(s)?;
        Ok(query)
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
    let (input, left) = parse_init(input)?;
    let (input, opt) = opt(preceded(space::around(char(',')), parse_split))(input)?;
    if let Some(right) = opt {
        Ok((input, Query::Split(Box::new(Split(left, right)))))
    } else {
        Ok((input, left))
    }
}

pub(crate) fn parse_init(input: &str) -> IResult<&str, Query, ParseError> {
    space::around(alt((
        chain(alt((
            parse_index_shorthand,
            map(Construct::parse, Query::Contruct),
            preceded(char('.'), alt((parse_index, parse_iterator))),
        ))),
        value(Query::Recurse, tag("..")),
        value(Query::Identity, char('.')),
    )))(input)
}

pub(crate) fn parse_chain(input: &str) -> IResult<&str, Query, ParseError> {
    chain(alt((parse_index_shorthand, parse_index, parse_iterator)))(input)
}

fn parse_index(input: &str) -> IResult<&str, Query, ParseError> {
    optional(map(Index::parse, Query::Index))(input)
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
mod test {
    use crate::{
        combinator::Optional,
        construction::{Construct, Key},
        range::Range,
    };

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
        assert_eq!(
            Query::Optional(Box::new(Optional(Query::Iterator))),
            ".[]?".parse().unwrap()
        );
        assert_eq!(
            Query::Chain(Box::new(Chain(
                Query::Iterator,
                Query::Chain(Box::new(Chain(Query::Iterator, Query::Iterator)))
            ))),
            ".[][][]".parse().unwrap()
        );
    }

    #[test]
    fn object_index() {
        assert!("foo".parse::<Query>().is_err());
        assert!("..foo".parse::<Query>().is_err());
        assert!(".f$$".parse::<Query>().is_err());
        assert!(".[f$$]".parse::<Query>().is_err());
        assert!(".[foo]".parse::<Query>().is_err());
        assert!(".[\"foo]".parse::<Query>().is_err());
        assert!(".[foo\"]".parse::<Query>().is_err());

        assert_eq!(
            Query::Index(Index::String("foo".to_string())),
            ".foo".parse().unwrap()
        );
        assert_eq!(
            Query::Optional(Box::new(Optional(Query::Index(Index::String(
                "foo".to_string()
            ))))),
            ".foo?".parse().unwrap()
        );
        assert_eq!(
            Query::Index(Index::String("foo".to_string())),
            ".[\"foo\"]".parse().unwrap()
        );
        assert_eq!(
            Query::Optional(Box::new(Optional(Query::Index(Index::String(
                "foo".to_string()
            ))))),
            ".[\"foo\"]?".parse().unwrap()
        );
        assert_eq!(
            Query::Chain(Box::new(Chain(
                Query::Index(Index::String("foo".to_string())),
                Query::Chain(Box::new(Chain(
                    Query::Index(Index::String("bar".to_string())),
                    Query::Index(Index::String("baz".to_string()))
                )))
            ))),
            ".foo.bar.baz".parse().unwrap()
        );
    }

    #[test]
    fn array_index() {
        assert!("[0]".parse::<Query>().is_err());
        assert!(".[a]".parse::<Query>().is_err());
        assert!("..[0]".parse::<Query>().is_err());
        assert!(".[0].[0]".parse::<Query>().is_err());

        assert_eq!(Query::Index(Index::Integer(0)), ".[0]".parse().unwrap());
        assert_eq!(Query::Index(Index::Integer(-1)), ".[-1]".parse().unwrap());
        assert_eq!(
            Query::Optional(Box::new(Optional(Query::Index(Index::Integer(0))))),
            ".[0]?".parse().unwrap()
        );
        assert_eq!(
            Query::Index(Index::Integer(9001)),
            ".[9001]".parse().unwrap()
        );
        assert_eq!(
            Query::Chain(Box::new(Chain(
                Query::Index(Index::Integer(5)),
                Query::Chain(Box::new(Chain(
                    Query::Index(Index::Integer(8)),
                    Query::Index(Index::Integer(13))
                )))
            ))),
            ".[5][8][13]".parse().unwrap()
        );
    }

    #[test]
    fn slice() {
        assert!(".[:]".parse::<Query>().is_err());
        assert!(".[1::2]".parse::<Query>().is_err());
        assert!(".[:2:]".parse::<Query>().is_err());
        assert!(".[--2]".parse::<Query>().is_err());
        assert!(".[-2:4:]".parse::<Query>().is_err());
        assert!(".[a]".parse::<Query>().is_err());
        assert!("..[1:2]".parse::<Query>().is_err());

        assert_eq!(
            Query::Index(Index::Slice(Range::new((-1, 2)))),
            ".[-1:2]".parse().unwrap()
        );
        assert_eq!(
            Query::Index(Index::Slice(Range::upper(2))),
            ".[:2]".parse().unwrap()
        );
        assert_eq!(
            Query::Optional(Box::new(Optional(Query::Index(Index::Slice(
                Range::lower(1)
            ))))),
            ".[1:]?".parse().unwrap()
        );
        assert_eq!(
            Query::Index(Index::Slice(Range::new((9001, -9001)))),
            ".[9001:-9001]".parse().unwrap()
        );
    }

    #[test]
    fn split() {
        assert!(",.".parse::<Query>().is_err());
        assert!(".,,.".parse::<Query>().is_err());
        assert!(",,".parse::<Query>().is_err());

        assert_eq!(
            Query::Split(Box::new(Split(
                Query::Identity,
                Query::Split(Box::new(Split(Query::Identity, Query::Identity)))
            ))),
            ".,.,.".parse().unwrap()
        );
        assert_eq!(
            Query::Split(Box::new(Split(
                Query::Index(Index::String("foo".to_string())),
                Query::Index(Index::String("bar".to_string()))
            ))),
            ".foo,.bar".parse().unwrap()
        );
    }

    #[test]
    fn pipe() {
        assert!("|.".parse::<Query>().is_err());
        assert!(".||.".parse::<Query>().is_err());
        assert!("|".parse::<Query>().is_err());

        assert_eq!(
            Query::Chain(Box::new(Chain(
                Query::Identity,
                Query::Chain(Box::new(Chain(Query::Identity, Query::Identity)))
            ))),
            ".|.|.".parse().unwrap()
        );
        assert_eq!(
            Query::Chain(Box::new(Chain(
                Query::Index(Index::String("foo".to_string())),
                Query::Index(Index::String("bar".to_string()))
            ))),
            ".foo|.bar".parse().unwrap()
        );
    }

    #[test]
    fn array_construction() {
        assert!("[".parse::<Query>().is_err());
        assert!("]".parse::<Query>().is_err());
        assert!("].[".parse::<Query>().is_err());
        assert!("[]".parse::<Query>().is_err()); // TODO: Probably should be allowed

        assert_eq!(
            Query::Contruct(Construct::Array(Box::new(Query::Identity))),
            "[.]".parse().unwrap()
        );
        assert_eq!(
            Query::Contruct(Construct::Array(Box::new(Query::Split(Box::new(Split(
                Query::Index(Index::String("foo".to_string())),
                Query::Index(Index::String("bar".to_string()))
            )))))),
            "[.foo,.bar]".parse().unwrap()
        );
    }

    #[test]
    fn object_construction() {
        assert!("{".parse::<Query>().is_err());
        assert!("}".parse::<Query>().is_err());
        assert!("}{".parse::<Query>().is_err());
        assert!("{:}".parse::<Query>().is_err());
        assert!("{foo:}".parse::<Query>().is_err());
        assert!("{:.}".parse::<Query>().is_err());
        assert!("{.:.}".parse::<Query>().is_err());
        assert!("{():.}".parse::<Query>().is_err());

        assert_eq!(
            Query::Contruct(Construct::Object(vec![])),
            "{}".parse().unwrap()
        );
        assert_eq!(
            Query::Contruct(Construct::Object(vec![
                Construct::shorthand("foo".to_string()),
                (
                    Key::Simple("bar".to_string()),
                    Query::Index(Index::String("bar".to_string()))
                ),
                (
                    Key::Query(Query::Index(Index::String("baz".to_string()))),
                    Query::Iterator
                )
            ])),
            "{foo,bar:.bar,(.baz):.[]}".parse().unwrap()
        );
    }
}
