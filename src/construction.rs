use crate::{
    index::Index,
    parse::{parse_init, parse_pipe, ParseError, Parseable},
    query::{Executable, Query},
    space, type_str, QueryError, QueryResult,
};
use itertools::Itertools;
use nom::{
    branch::alt,
    bytes::complete::take_while1,
    character::complete::{alphanumeric1, char},
    combinator::map,
    multi::separated_list0,
    sequence::{delimited, separated_pair},
    IResult,
};
use serde_json::Value;

#[derive(Debug, PartialEq, Clone)]
pub enum Construct {
    Array(Box<Query>),
    Object(Vec<(Key, Query)>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Key {
    Simple(String),
    Query(Query),
}

impl Key {
    fn execute(&self, value: &Value) -> Result<Vec<String>, QueryError> {
        let keys = match self {
            Key::Simple(s) => vec![s.clone()],
            Key::Query(inner) => {
                let mut keys = Vec::new();
                for k in inner.execute(value)? {
                    match k {
                        Value::String(s) => keys.push(s),
                        vv => return Err(QueryError::ObjectKey(type_str(&vv))),
                    }
                }
                keys
            }
        };
        Ok(keys)
    }
}

impl Construct {
    pub fn shorthand(s: String) -> (Key, Query) {
        let k = Key::Simple(s.clone());
        let q = Query::Index(Index::String(s));
        (k, q)
    }
}

impl Executable for Construct {
    fn execute(&self, value: &Value) -> QueryResult {
        match self {
            Construct::Array(inner) => construct_array(value, inner),
            Construct::Object(kvs) => construct_object(value, kvs),
        }
    }
}

fn construct_array(v: &Value, inner: &Query) -> QueryResult {
    Ok(vec![Value::Array(inner.execute(v)?)])
}

fn construct_object(value: &Value, kvs: &[(Key, Query)]) -> QueryResult {
    Ok(kvs
        .iter()
        .map(|(k, v)| (k.execute(value), v.execute(value)))
        .map(|(kr, vr)| kr.and_then(|ks| vr.map(|vs| (ks, vs))))
        .collect::<Result<Vec<(Vec<String>, Vec<Value>)>, _>>()? // Unwrap pairs of results into just pairs of vectors
        .into_iter() // At this point, each of key and value might have been evaluated to to many values
        .map(|(ks, vs)| ks.into_iter().cartesian_product(vs)) // Get all combinations for each pair
        .multi_cartesian_product() // Get all combinations of different pairs
        .map(|pairs| pairs.into_iter().collect())
        .map(Value::Object)
        .collect())
}

impl Parseable for Construct {
    fn parser(input: &str) -> IResult<&str, Construct, ParseError> {
        alt((parse_array, parse_object))(input)
    }
}

fn parse_array(input: &str) -> IResult<&str, Construct, ParseError> {
    let (input, inner) = delimited(char('['), space::around(parse_pipe), char(']'))(input)?;
    Ok((input, Construct::Array(Box::new(inner))))
}

fn parse_object(input: &str) -> IResult<&str, Construct, ParseError> {
    let (input, kvs) = delimited(
        char('{'),
        space::around(separated_list0(
            char(','),
            space::around(alt((
                separated_pair(
                    alt((
                        map(delimited(char('('), parse_init, char(')')), Key::Query),
                        map(
                            alt((
                                alphanumeric1,
                                delimited(char('"'), take_while1(|c| c != '"'), char('"')),
                            )),
                            |s: &str| Key::Simple(s.to_string()),
                        ),
                    )),
                    space::around(char(':')),
                    parse_init,
                ),
                map(alphanumeric1, |s: &str| Construct::shorthand(s.to_string())),
            ))),
        )),
        char('}'),
    )(input)?;
    Ok((input, Construct::Object(kvs)))
}

#[cfg(test)]
mod tests {
    use crate::combinator::Split;

    use super::*;

    #[test]
    fn array_construction() {
        assert!(Construct::parse("[").is_err());
        assert!(Construct::parse("]").is_err());
        assert!(Construct::parse("].[").is_err());
        assert!(Construct::parse("[]").is_err()); // TODO: Probably should be allowed

        assert_eq!(
            Construct::Array(Box::new(Query::Identity)),
            Construct::parse("[.]").unwrap()
        );
        assert_eq!(
            Construct::Array(Box::new(Query::Split(Box::new(Split(
                Query::Index(Index::String("foo".to_string())),
                Query::Index(Index::String("bar".to_string()))
            ))))),
            Construct::parse("[.foo,.bar]").unwrap()
        );
    }

    #[test]
    fn object_construction() {
        assert!(Construct::parse("{").is_err());
        assert!(Construct::parse("}").is_err());
        assert!(Construct::parse("}{").is_err());
        assert!(Construct::parse("{:}").is_err());
        assert!(Construct::parse("{foo:}").is_err());
        assert!(Construct::parse("{:.}").is_err());
        assert!(Construct::parse("{.:.}").is_err());
        assert!(Construct::parse("{():.}").is_err());

        assert_eq!(Construct::Object(vec![]), Construct::parse("{}").unwrap());
        assert_eq!(
            Construct::Object(vec![
                Construct::shorthand("foo".to_string()),
                (
                    Key::Simple("bar".to_string()),
                    Query::Index(Index::String("bar".to_string()))
                ),
                (
                    Key::Query(Query::Index(Index::String("baz".to_string()))),
                    Query::Iterator
                )
            ]),
            Construct::parse("{foo,bar:.bar,(.baz):.[]}").unwrap()
        );
    }
}
