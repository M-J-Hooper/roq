use nom::{character::complete::char, combinator::opt, IResult};
use serde_json::Value;

use crate::{
    empty,
    parse::{parse_chain, ParseError},
    query::{iterate_results, iterate_values, Executable, Query},
    QueryResult,
};

#[derive(Debug, PartialEq, Clone)]
pub struct Split(pub Query, pub Query);

impl Executable for Split {
    fn execute(&self, value: &Value) -> QueryResult {
        iterate_results(vec![self.0.execute(value), self.1.execute(value)])
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Chain(pub Query, pub Query);

impl Executable for Chain {
    fn execute(&self, value: &Value) -> QueryResult {
        iterate_values(self.0.execute(value)?.iter(), &self.1)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Optional(pub Query);

impl Executable for Optional {
    fn execute(&self, value: &Value) -> QueryResult {
        match self.0.execute(value) {
            Ok(v) => Ok(v),
            Err(_) => empty(),
        }
    }
}

pub(crate) fn optional<'a, F>(
    mut f: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, Query, ParseError>
where
    F: FnMut(&'a str) -> IResult<&'a str, Query, ParseError>,
{
    move |input: &'a str| {
        let (input, q) = f(input)?;
        let (input, opt) = opt(char('?'))(input)?;
        let q = match opt {
            Some(_) => Query::Optional(Box::new(Optional(q))),
            None => q,
        };
        Ok((input, q))
    }
}

pub(crate) fn chain<'a, F>(mut f: F) -> impl FnMut(&'a str) -> IResult<&'a str, Query, ParseError>
where
    F: FnMut(&'a str) -> IResult<&'a str, Query, ParseError>,
{
    move |input: &'a str| {
        let (input, q) = f(input)?;
        let (input, next) = opt(parse_chain)(input)?;
        let q = match next {
            Some(qq) => Query::Chain(Box::new(Chain(q, qq))),
            None => q,
        };
        Ok((input, q))
    }
}

#[cfg(test)]
mod tests {
    use crate::{index::Index, parse::Parseable, range::Range};

    use super::*;

    #[test]
    fn parse_split() {
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
    fn parse_pipe_chain() {
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
    fn parse_optional() {
        assert!(Query::parse(".?").is_err());
        assert!(Query::parse(".[]??").is_err());
        assert!(Query::parse("?").is_err());
        assert!(Query::parse(".[0] ?").is_err());

        assert_eq!(
            Query::Optional(Box::new(Optional(Query::Index(Index::String(
                "foo".to_string()
            ))))),
            Query::parse(".foo?").unwrap()
        );
        assert_eq!(
            Query::Optional(Box::new(Optional(Query::Index(Index::String(
                "foo".to_string()
            ))))),
            Query::parse(".[\"foo\"]?").unwrap()
        );
        assert_eq!(
            Query::Optional(Box::new(Optional(Query::Index(Index::Integer(0))))),
            Query::parse(".[0]?").unwrap()
        );
        assert_eq!(
            Query::Optional(Box::new(Optional(Query::Index(Index::Slice(
                Range::lower(1)
            ))))),
            Query::parse(".[1:]?").unwrap()
        );
        assert_eq!(
            Query::Optional(Box::new(Optional(Query::Iterator))),
            ".[]?".parse().unwrap()
        );
    }

    #[test]
    fn parse_index_chain() {
        assert!(Query::parse(".foo.[0]").is_err());
        assert!(Query::parse(".foo .foo").is_err());
        assert!(Query::parse(".[0].[0]").is_err());

        assert_eq!(
            Query::Chain(Box::new(Chain(
                Query::Index(Index::String("foo".to_string())),
                Query::Chain(Box::new(Chain(
                    Query::Index(Index::String("bar".to_string())),
                    Query::Index(Index::String("baz".to_string()))
                )))
            ))),
            Query::parse(".foo.bar.baz").unwrap()
        );

        assert_eq!(
            Query::Chain(Box::new(Chain(
                Query::Index(Index::Integer(5)),
                Query::Chain(Box::new(Chain(
                    Query::Index(Index::Integer(8)),
                    Query::Index(Index::Integer(13))
                )))
            ))),
            Query::parse(".[5][8][13]").unwrap()
        );
    }

    #[test]
    fn parse_iterator_chain() {
        assert!(Query::parse(".[].[]").is_err());
        assert!(Query::parse(".[] []").is_err());
        assert!(Query::parse(".[] .[]").is_err());

        assert_eq!(
            Query::Chain(Box::new(Chain(
                Query::Iterator,
                Query::Chain(Box::new(Chain(Query::Iterator, Query::Iterator)))
            ))),
            ".[][][]".parse().unwrap()
        );
    }
}
