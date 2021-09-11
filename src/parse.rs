use crate::query::{Index, Query};
use crate::range::Range;
use nom::combinator::{all_consuming, success};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::{complete::char, is_alphabetic, is_digit},
    combinator::{map, map_res, opt, value},
    multi::separated_list1,
    sequence::{delimited, preceded, separated_pair, terminated},
    IResult,
};
use thiserror::Error;

type ParseResult<'a> = Result<Query, ParseError>;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Leftover characters after parsing: {0}")]
    LeftoverCharacters(String),
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

impl<E: std::fmt::Debug> From<nom::Err<E>> for ParseError {
    fn from(err: nom::Err<E>) -> Self {
        let s = match err {
            nom::Err::Incomplete(n) => format!("{:?}", n),
            nom::Err::Error(e) | nom::Err::Failure(e) => format!("{:?}", e),
        };
        ParseError::InvalidFormat(s)
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

    let (leftover, query) = all_consuming(pipe)(input.as_bytes())?;
    assert!(leftover.is_empty());
    Ok(query)
}

fn pipe(input: &[u8]) -> IResult<&[u8], Query> {
    let (input, curr) = split(input)?;
    let (input, opt) = opt(preceded(char('|'), pipe))(input)?;
    if let Some(next) = opt {
        Ok((input, Query::Pipe(Box::new(curr), Box::new(next))))
    } else {
        Ok((input, curr))
    }
}

fn split(input: &[u8]) -> IResult<&[u8], Query> {
    let (input, qs) = separated_list1(char(','), init_parser)(input)?;
    if qs.len() == 1 {
        Ok((input, qs.into_iter().nth(0).unwrap()))
    } else {
        Ok((input, Query::Spliterator(qs)))
    }
}

fn init_parser(input: &[u8]) -> IResult<&[u8], Query> {
    alt((
        object_index,
        preceded(char('.'), slice),
        preceded(char('.'), array_index),
        preceded(char('.'), iterator),
        value(Query::Identity, char('.')),
    ))(input)
}

fn parser(input: &[u8]) -> IResult<&[u8], Query> {
    alt((
        object_index,
        slice,
        array_index,
        iterator,
        success(Query::Identity),
    ))(input)
}

fn iterator(input: &[u8]) -> IResult<&[u8], Query> {
    let (input, _) = tag("[]")(input)?;
    let (input, opt) = opt(char('?'))(input)?;
    let (input, next) = parser(input)?;
    Ok((input, Query::Iterator(opt.is_some(), Box::new(next))))
}

fn object_index(input: &[u8]) -> IResult<&[u8], Query> {
    let (input, _) = char('.')(input)?;
    let (input, bytes) = alt((
        take_while1(is_alphabetic),
        delimited(tag("[\""), take_while1(is_alphabetic), tag("\"]")), //FIXME: Escaped string
    ))(input)?;
    let (input, opt) = opt(char('?'))(input)?;
    let (input, next) = parser(input)?;

    let i = std::str::from_utf8(bytes).unwrap().to_string(); //FIXME: Handle bad utf8
    Ok((
        input,
        Query::Index(Index::String(i), opt.is_some(), Box::new(next)),
    ))
}

fn array_index(input: &[u8]) -> IResult<&[u8], Query> {
    let (input, i) = delimited(char('['), num, char(']'))(input)?;
    let (input, opt) = opt(char('?'))(input)?;
    let (input, next) = parser(input)?;

    Ok((
        input,
        Query::Index(Index::Integer(i), opt.is_some(), Box::new(next)),
    ))
}

fn slice(input: &[u8]) -> IResult<&[u8], Query> {
    let (input, r) = delimited(
        char('['),
        alt((
            map(separated_pair(num, char(':'), num), Range::new),
            map(preceded(char(':'), num), Range::upper),
            map(terminated(num, char(':')), Range::lower),
        )),
        char(']'),
    )(input)?;
    let (input, opt) = opt(char('?'))(input)?;
    let (input, next) = parser(input)?;

    Ok((
        input,
        Query::Index(Index::Slice(r), opt.is_some(), Box::new(next)),
    ))
}

fn num(input: &[u8]) -> IResult<&[u8], isize> {
    let (input, neg) = opt(char('-'))(input)?;
    let (input, mut i) = map_res(
        map_res(take_while1(is_digit), std::str::from_utf8),
        std::str::FromStr::from_str,
    )(input)?;

    if neg.is_some() {
        i *= -1;
    }
    Ok((input, i))
}

#[cfg(test)]
mod test {
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

        assert_eq!(
            Query::Iterator(false, Box::new(Query::Identity)),
            parse(".[]").unwrap()
        );
        assert_eq!(
            Query::Iterator(true, Box::new(Query::Identity)),
            parse(".[]?").unwrap()
        );
        assert_eq!(
            Query::Iterator(
                false,
                Box::new(Query::Iterator(
                    false,
                    Box::new(Query::Iterator(false, Box::new(Query::Identity)))
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
            Query::Index(
                Index::String("foo".to_string()),
                false,
                Box::new(Query::Identity)
            ),
            parse(".foo").unwrap()
        );
        assert_eq!(
            Query::Index(
                Index::String("foo".to_string()),
                true,
                Box::new(Query::Identity)
            ),
            parse(".foo?").unwrap()
        );
        assert_eq!(
            Query::Index(
                Index::String("foo".to_string()),
                false,
                Box::new(Query::Identity)
            ),
            parse(".[\"foo\"]").unwrap()
        );
        assert_eq!(
            Query::Index(
                Index::String("foo".to_string()),
                true,
                Box::new(Query::Identity)
            ),
            parse(".[\"foo\"]?").unwrap()
        );
        assert_eq!(
            Query::Index(
                Index::String("foo".to_string()),
                false,
                Box::new(Query::Index(
                    Index::String("bar".to_string()),
                    false,
                    Box::new(Query::Index(
                        Index::String("baz".to_string()),
                        false,
                        Box::new(Query::Identity)
                    ))
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

        assert_eq!(
            Query::Index(Index::Integer(0), false, Box::new(Query::Identity)),
            parse(".[0]").unwrap()
        );
        assert_eq!(
            Query::Index(Index::Integer(-1), false, Box::new(Query::Identity)),
            parse(".[-1]").unwrap()
        );
        assert_eq!(
            Query::Index(Index::Integer(0), true, Box::new(Query::Identity)),
            parse(".[0]?").unwrap()
        );
        assert_eq!(
            Query::Index(Index::Integer(9001), false, Box::new(Query::Identity)),
            parse(".[9001]").unwrap()
        );
        assert_eq!(
            Query::Index(
                Index::Integer(5),
                false,
                Box::new(Query::Index(
                    Index::Integer(8),
                    false,
                    Box::new(Query::Index(
                        Index::Integer(13),
                        false,
                        Box::new(Query::Identity)
                    ))
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
            Query::Index(
                Index::Slice(Range::new((-1, 2))),
                false,
                Box::new(Query::Identity)
            ),
            parse(".[-1:2]").unwrap()
        );
        assert_eq!(
            Query::Index(
                Index::Slice(Range::upper(2)),
                false,
                Box::new(Query::Identity)
            ),
            parse(".[:2]").unwrap()
        );
        assert_eq!(
            Query::Index(
                Index::Slice(Range::lower(1)),
                true,
                Box::new(Query::Identity)
            ),
            parse(".[1:]?").unwrap()
        );
        assert_eq!(
            Query::Index(
                Index::Slice(Range::new((9001, -9001))),
                false,
                Box::new(Query::Identity)
            ),
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
            Query::Spliterator(vec![Query::Identity, Query::Identity, Query::Identity]),
            parse(".,.,.").unwrap()
        );
        assert_eq!(
            Query::Spliterator(vec![
                Query::Index(
                    Index::String("foo".to_string()),
                    false,
                    Box::new(Query::Identity)
                ),
                Query::Index(
                    Index::String("bar".to_string()),
                    false,
                    Box::new(Query::Identity)
                ),
            ]),
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
            Query::Pipe(
                Box::new(Query::Identity),
                Box::new(Query::Pipe(
                    Box::new(Query::Identity),
                    Box::new(Query::Identity)
                ))
            ),
            parse(".|.|.").unwrap()
        );
        assert_eq!(
            Query::Pipe(
                Box::new(Query::Index(
                    Index::String("foo".to_string()),
                    false,
                    Box::new(Query::Identity)
                )),
                Box::new(Query::Index(
                    Index::String("bar".to_string()),
                    false,
                    Box::new(Query::Identity)
                ))
            ),
            parse(".foo|.bar").unwrap()
        );
    }
}
