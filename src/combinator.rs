use nom::{IResult, combinator::opt, character::complete::char};
use serde_json::Value;

use crate::{QueryResult, empty, parse::{ParseError, parse_chain}, query::{iterate_results, iterate_values, Executable, Query}};

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

pub(crate) fn optional<'a, F>(mut f: F) -> impl FnMut(&'a str) -> IResult<&'a str, Query, ParseError>
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
