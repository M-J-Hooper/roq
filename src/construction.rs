use crate::{
    index::Index,
    parse::{parse_init, parse_pipe, ParseError, Parseable},
    query::{Executable, Query},
    space, type_str, QueryError, QueryResult,
};
use nom::{
    branch::alt,
    character::complete::{alphanumeric1, char},
    combinator::map,
    multi::separated_list0,
    sequence::{delimited, separated_pair},
    IResult,
};
use serde_json::{Map, Value};

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

fn construct_object(v: &Value, kvs: &Vec<(Key, Query)>) -> QueryResult {
    let mut eval_keys = Vec::new();
    let mut eval_values = Vec::new();
    for kv in kvs {
        let keys = match &kv.0 {
            Key::Simple(s) => vec![s.clone()],
            Key::Query(inner) => {
                let mut keys = Vec::new();
                for k in inner.execute(v)? {
                    match k {
                        Value::String(s) => keys.push(s),
                        vv => return Err(QueryError::ObjectKey(type_str(&vv))),
                    }
                }
                keys
            }
        };
        let values = kv.1.execute(v)?;

        eval_keys.push(keys);
        eval_values.push(values);
    }

    // For each key-value query, there is a vector of permutations
    let mut combined_perms = Vec::new();
    for i in 0..eval_keys.len() {
        let mut single_perms = Vec::new();
        for k in &eval_keys[i] {
            for v in &eval_values[i] {
                single_perms.push((k.clone(), v.clone()));
            }
        }
        combined_perms.push(single_perms);
    }

    // Now get all combinations of these permutations
    let mut objs = Vec::new();
    let n = combined_perms.iter().fold(1, |i, v| i * v.len());
    for i in 0..n {
        let mut map = Map::new();
        let mut x = i;
        for j in 0..combined_perms.len() {
            let perms = &combined_perms[j];
            let l = perms.len();

            let k = x % l;
            let (key, value) = &perms[k];
            map.insert(key.clone(), value.clone());

            x /= l;
        }
        objs.push(Value::Object(map));
    }
    Ok(objs)
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
                        map(alphanumeric1, |s: &str| Key::Simple(s.to_string())),
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
mod test {
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
