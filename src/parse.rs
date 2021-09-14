use crate::construction::{self};
use crate::index::{self, Index};
use crate::query::Query;
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

type ParseResult<'a> = Result<Query, ParseError>;

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
        parse(s)
    }
}

fn parse(input: &str) -> ParseResult {
    if input.is_empty() {
        return Ok(Query::Empty);
    }
    let (_, query) = all_consuming(pipe)(input)?;
    Ok(query)
}

pub(crate) fn pipe(input: &str) -> IResult<&str, Query, ParseError> {
    let (input, curr) = split(input)?;
    let (input, opt) = opt(preceded(char('|'), pipe))(input)?;
    if let Some(next) = opt {
        Ok((input, Query::Chain(Box::new(curr), Box::new(next))))
    } else {
        Ok((input, curr))
    }
}

fn split(input: &str) -> IResult<&str, Query, ParseError> {
    let (input, curr) = init_parser(input)?;
    let (input, opt) = opt(preceded(char(','), split))(input)?;
    if let Some(next) = opt {
        Ok((input, Query::Split(Box::new(curr), Box::new(next))))
    } else {
        Ok((input, curr))
    }
}

pub(crate) fn init_parser(input: &str) -> IResult<&str, Query, ParseError> {
    chain(alt((
        optional(bare_object_index),
        map(construction::parse, Query::Contruct),
        preceded(char('.'), alt((optional(index), optional(iterator)))),
        value(Query::Identity, char('.')),
    )))(input)
}

fn parser(input: &str) -> IResult<&str, Query, ParseError> {
    chain(alt((
        optional(bare_object_index),
        optional(index),
        optional(iterator),
    )))(input)
}

fn index(input: &str) -> IResult<&str, Query, ParseError> {
    map(index::parse, Query::Index)(input)
}

fn iterator(input: &str) -> IResult<&str, Query, ParseError> {
    value(Query::Iterator, tag("[]"))(input)
}

fn bare_object_index(input: &str) -> IResult<&str, Query, ParseError> {
    let (input, i) = preceded(char('.'), alphanumeric1)(input)?;
    Ok((input, Query::Index(Index::String(i.to_string()))))
}

fn optional<'a, F>(mut f: F) -> impl FnMut(&'a str) -> IResult<&'a str, Query, ParseError>
where
    F: FnMut(&'a str) -> IResult<&'a str, Query, ParseError>,
{
    move |input: &'a str| {
        let (input, q) = f(input)?;
        let (input, opt) = opt(char('?'))(input)?;
        let q = match opt {
            Some(_) => Query::Optional(Box::new(q)),
            None => q,
        };
        Ok((input, q))
    }
}

fn chain<'a, F>(mut f: F) -> impl FnMut(&'a str) -> IResult<&'a str, Query, ParseError>
where
    F: FnMut(&'a str) -> IResult<&'a str, Query, ParseError>,
{
    move |input: &'a str| {
        let (input, q) = f(input)?;
        let (input, next) = opt(parser)(input)?;
        let q = match next {
            Some(qq) => Query::Chain(Box::new(q), Box::new(qq)),
            None => q,
        };
        Ok((input, q))
    }
}

#[cfg(test)]
mod test {
    use crate::{
        construction::{Construct, Key},
        range::Range,
    };

    use super::*;

    #[test]
    fn simple() {
        assert!(parse("...").is_err());
        assert_eq!(Query::Identity, parse(".").unwrap());
        assert_eq!(Query::Empty, parse("").unwrap());
    }

    #[test]
    fn iterator() {
        assert!(parse("[]").is_err());
        assert!(parse(".[").is_err());
        assert!(parse(".]").is_err());
        assert!(parse(".[].[]").is_err());

        assert_eq!(Query::Iterator, parse(".[]").unwrap());
        assert_eq!(
            Query::Optional(Box::new(Query::Iterator)),
            parse(".[]?").unwrap()
        );
        assert_eq!(
            Query::Chain(
                Box::new(Query::Iterator),
                Box::new(Query::Chain(
                    Box::new(Query::Iterator),
                    Box::new(Query::Iterator)
                ))
            ),
            parse(".[][][]").unwrap()
        );
    }

    #[test]
    fn object_index() {
        assert!(parse("foo").is_err());
        assert!(parse(".f$$").is_err());
        assert!(parse(".[f$$]").is_err());
        assert!(parse(".[foo]").is_err());
        assert!(parse(".[\"foo]").is_err());
        assert!(parse(".[foo\"]").is_err());

        assert_eq!(
            Query::Index(Index::String("foo".to_string())),
            parse(".foo").unwrap()
        );
        assert_eq!(
            Query::Optional(Box::new(Query::Index(Index::String("foo".to_string()),))),
            parse(".foo?").unwrap()
        );
        assert_eq!(
            Query::Index(Index::String("foo".to_string())),
            parse(".[\"foo\"]").unwrap()
        );
        assert_eq!(
            Query::Optional(Box::new(Query::Index(Index::String("foo".to_string()),))),
            parse(".[\"foo\"]?").unwrap()
        );
        assert_eq!(
            Query::Chain(
                Box::new(Query::Index(Index::String("foo".to_string()))),
                Box::new(Query::Chain(
                    Box::new(Query::Index(Index::String("bar".to_string()))),
                    Box::new(Query::Index(Index::String("baz".to_string())))
                ))
            ),
            parse(".foo.bar.baz").unwrap()
        );
    }

    #[test]
    fn array_index() {
        assert!(parse("[0]").is_err());
        assert!(parse(".[a]").is_err());
        assert!(parse(".[0].[0]").is_err());

        assert_eq!(Query::Index(Index::Integer(0)), parse(".[0]").unwrap());
        assert_eq!(Query::Index(Index::Integer(-1)), parse(".[-1]").unwrap());
        assert_eq!(
            Query::Optional(Box::new(Query::Index(Index::Integer(0)))),
            parse(".[0]?").unwrap()
        );
        assert_eq!(
            Query::Index(Index::Integer(9001)),
            parse(".[9001]").unwrap()
        );
        assert_eq!(
            Query::Chain(
                Box::new(Query::Index(Index::Integer(5))),
                Box::new(Query::Chain(
                    Box::new(Query::Index(Index::Integer(8))),
                    Box::new(Query::Index(Index::Integer(13)))
                ))
            ),
            parse(".[5][8][13]").unwrap()
        );
    }

    #[test]
    fn slice() {
        assert!(parse(".[:]").is_err());
        assert!(parse(".[1::2]").is_err());
        assert!(parse(".[:2:]").is_err());
        assert!(parse(".[--2]").is_err());
        assert!(parse(".[-2:4:]").is_err());
        assert!(parse(".[:-2:4]").is_err());

        assert_eq!(
            Query::Index(Index::Slice(Range::new((-1, 2)))),
            parse(".[-1:2]").unwrap()
        );
        assert_eq!(
            Query::Index(Index::Slice(Range::upper(2))),
            parse(".[:2]").unwrap()
        );
        assert_eq!(
            Query::Optional(Box::new(Query::Index(Index::Slice(Range::lower(1))))),
            parse(".[1:]?").unwrap()
        );
        assert_eq!(
            Query::Index(Index::Slice(Range::new((9001, -9001)))),
            parse(".[9001:-9001]").unwrap()
        );
    }

    #[test]
    fn split() {
        assert!(parse(",.").is_err());
        assert!(parse(".,,.").is_err());
        assert!(parse(",,").is_err());
        assert!(parse("., .").is_err()); // TODO: Handle whitespace

        assert_eq!(
            Query::Split(
                Box::new(Query::Identity),
                Box::new(Query::Split(
                    Box::new(Query::Identity),
                    Box::new(Query::Identity)
                ))
            ),
            parse(".,.,.").unwrap()
        );
        assert_eq!(
            Query::Split(
                Box::new(Query::Index(Index::String("foo".to_string()))),
                Box::new(Query::Index(Index::String("bar".to_string())))
            ),
            parse(".foo,.bar").unwrap()
        );
    }

    #[test]
    fn pipe() {
        assert!(parse("|.").is_err());
        assert!(parse(".||.").is_err());
        assert!(parse("|").is_err());
        assert!(parse("| .").is_err()); // TODO: Handle whitespace

        assert_eq!(
            Query::Chain(
                Box::new(Query::Identity),
                Box::new(Query::Chain(
                    Box::new(Query::Identity),
                    Box::new(Query::Identity)
                ))
            ),
            parse(".|.|.").unwrap()
        );
        assert_eq!(
            Query::Chain(
                Box::new(Query::Index(Index::String("foo".to_string()))),
                Box::new(Query::Index(Index::String("bar".to_string())))
            ),
            parse(".foo|.bar").unwrap()
        );
    }

    #[test]
    fn array_construction() {
        assert!(parse("[").is_err());
        assert!(parse("]").is_err());
        assert!(parse("].[").is_err());
        assert!(parse("[]").is_err()); // TODO: Probably should be allowed

        assert_eq!(
            Query::Contruct(Construct::Array(Box::new(Query::Identity)),),
            parse("[.]").unwrap()
        );
        assert_eq!(
            Query::Contruct(Construct::Array(Box::new(Query::Split(
                Box::new(Query::Index(Index::String("foo".to_string()))),
                Box::new(Query::Index(Index::String("bar".to_string())))
            )))),
            parse("[.foo,.bar]").unwrap()
        );
    }

    #[test]
    fn object_construction() {
        assert!(parse("{").is_err());
        assert!(parse("}").is_err());
        assert!(parse("}{").is_err());
        assert!(parse("{:}").is_err());
        assert!(parse("{foo:}").is_err());
        assert!(parse("{:.}").is_err());
        assert!(parse("{.:.}").is_err());

        assert_eq!(
            Query::Contruct(Construct::Object(vec![])),
            parse("{}").unwrap()
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
            parse("{foo,bar:.bar,(.baz):.[]}").unwrap()
        );
    }
}
