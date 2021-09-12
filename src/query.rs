use crate::range::Range;
use serde_json::{Map, Value};
use thiserror::Error;

type QueryResult = Result<Vec<Value>, QueryError>;

#[derive(Error, Debug)]
pub enum QueryError {
    #[error("Cannot index {0} with {1}")]
    Index(&'static str, &'static str),
    #[error("Cannot iterate over {0}")]
    Iterate(&'static str),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Query {
    Empty,
    Identity,
    Index(Index, bool, Box<Query>),
    Iterator(bool, Box<Query>),
    Spliterator(Box<Query>, Box<Query>),
    Pipe(Box<Query>, Box<Query>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Index {
    String(String),
    Integer(i32),
    Slice(Range),
}

impl Query {
    pub fn execute(&self, value: &Value) -> QueryResult {
        if value.is_null() {
            return null();
        }
        match self {
            Query::Empty => empty(),
            Query::Identity => single(value.clone()),
            Query::Index(i, opt, next) => check(index(value, i, next), *opt),
            Query::Iterator(opt, next) => check(iterate(value, next), *opt),
            Query::Spliterator(curr, next) => split(value, curr, next),
            Query::Pipe(curr, next) => pipe(value, curr, next),
        }
    }
}

fn single(value: Value) -> QueryResult {
    Ok(vec![value])
}
fn null() -> QueryResult {
    single(Value::Null)
}
fn empty() -> QueryResult {
    Ok(Vec::new())
}

fn check(r: QueryResult, opt: bool) -> QueryResult {
    if r.is_ok() || !opt {
        r
    } else {
        empty()
    }
}

fn index(v: &Value, i: &Index, next: &Query) -> QueryResult {
    match (v, i) {
        (Value::String(s), Index::Slice(r)) => {
            let range = r.normalize(s.len());
            let sliced = s[range].to_string();
            next.execute(&Value::String(sliced))
        }
        (Value::Array(vec), Index::Slice(r)) => {
            let range = r.normalize(vec.len());
            let sliced = vec[range].to_vec();
            next.execute(&Value::Array(sliced))
        }
        (Value::Object(map), Index::String(s)) => object_index(map, s, next),
        (Value::Array(arr), Index::Integer(i)) => array_index(arr, *i, next),
        (v, Index::String(_)) => Err(QueryError::Index(type_str(v), "string")),
        (v, Index::Integer(_)) => Err(QueryError::Index(type_str(v), "number")),
        (v, Index::Slice(_)) => Err(QueryError::Index(type_str(v), "slice")),
    }
}

fn pipe(v: &Value, curr: &Query, next: &Query) -> QueryResult {
    iterate_values(curr.execute(v)?.iter(), next)
}

fn split(v: &Value, curr: &Query, next: &Query) -> QueryResult {
    iterate_results(vec![curr.execute(v), next.execute(v)])
}

fn iterate(v: &Value, next: &Query) -> QueryResult {
    match v {
        Value::Array(arr) => iterate_values(arr, next),
        Value::Object(map) => iterate_values(map.values(), next),
        v => Err(QueryError::Iterate(type_str(v))),
    }
}

fn iterate_values<'a, I: IntoIterator<Item = &'a Value>>(iter: I, next: &Query) -> QueryResult {
    iterate_results(iter.into_iter().map(|vv| next.execute(vv)))
}

fn iterate_results<'a, I: IntoIterator<Item = QueryResult>>(iter: I) -> QueryResult {
    Ok(iter
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect())
}

fn object_index(map: &Map<String, Value>, s: &str, next: &Query) -> QueryResult {
    if let Some(vv) = map.get(s) {
        next.execute(vv)
    } else {
        null()
    }
}

fn array_index(arr: &Vec<Value>, i: i32, next: &Query) -> QueryResult {
    let index = if i < 0 {
        let j = -i as usize;
        if j >= arr.len() {
            return null();
        }
        arr.len() - j
    } else {
        i as usize
    };

    if let Some(vv) = arr.get(index) {
        next.execute(vv)
    } else {
        null()
    }
}

fn type_str(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod test {
    use super::*;
    // Tests are taken from examples at https://stedolan.github.io/jq/manual

    #[test]
    fn identity() {
        let q: Query = ".".parse().unwrap();
        let v: Value = serde_json::from_str(r#""Hello world!""#).unwrap();
        assert_eq!(r#""Hello world!""#, q.execute(&v).unwrap()[0].to_string());
    }

    #[test]
    fn object_index() {
        let q: Query = ".foo".parse().unwrap();
        let v: Value =
            serde_json::from_str(r#"{"foo": 42, "bar": "less interesting data"}"#).unwrap();
        assert_eq!(r#"42"#, q.execute(&v).unwrap()[0].to_string());

        let v: Value = serde_json::from_str(r#"{"notfoo": true, "alsonotfoo": false}"#).unwrap();
        assert_eq!(r#"null"#, q.execute(&v).unwrap()[0].to_string());

        let v: Value = serde_json::from_str(r#"{"foo": 42}"#).unwrap();
        assert_eq!(r#"42"#, q.execute(&v).unwrap()[0].to_string());
    }

    #[test]
    fn optional_object_index() {
        let q: Query = ".foo?".parse().unwrap();
        let v: Value =
            serde_json::from_str(r#"{"foo": 42, "bar": "less interesting data"}"#).unwrap();
        assert_eq!(r#"42"#, q.execute(&v).unwrap()[0].to_string());

        let v: Value = serde_json::from_str(r#"{"notfoo": true, "alsonotfoo": false}"#).unwrap();
        assert_eq!(r#"null"#, q.execute(&v).unwrap()[0].to_string());

        let q: Query = ".[\"foo\"]?".parse().unwrap();
        let v: Value = serde_json::from_str(r#"{"foo": 42}"#).unwrap();
        assert_eq!(r#"42"#, q.execute(&v).unwrap()[0].to_string());

        assert!("[.foo?]".parse::<Query>().is_err()); // TODO: Implement array construction
                                                      //let v: Value = serde_json::from_str(r#"[1,2]"#).unwrap();
                                                      //assert_eq!(r#"[]"#, q.execute(&v).unwrap()[0].to_string());
    }

    #[test]
    fn array_index() {
        let q: Query = ".[0]".parse().unwrap();
        let v: Value =
            serde_json::from_str(r#"[{"name":"JSON", "good":true},{"name":"XML", "good":false}]"#)
                .unwrap();
        assert_eq!(
            r#"{"good":true,"name":"JSON"}"#,
            q.execute(&v).unwrap()[0].to_string()
        );

        let q: Query = ".[2]".parse().unwrap();
        assert_eq!(r#"null"#, q.execute(&v).unwrap()[0].to_string());

        let q: Query = ".[-2]".parse::<Query>().unwrap();
        let v: Value = serde_json::from_str(r#"[1,2,3]"#).unwrap();
        assert_eq!(r#"2"#, q.execute(&v).unwrap()[0].to_string());
    }

    #[test]
    fn iterator() {
        let q: Query = ".[]".parse().unwrap();
        let v: Value =
            serde_json::from_str(r#"[{"name":"JSON", "good":true}, {"name":"XML", "good":false}]"#)
                .unwrap();
        let r = q.execute(&v).unwrap();
        assert_eq!(r#"{"good":true,"name":"JSON"}"#, r[0].to_string());
        assert_eq!(r#"{"good":false,"name":"XML"}"#, r[1].to_string());

        let v: Value = serde_json::from_str(r#"{"a": 1, "b": 1}"#).unwrap();
        let r = q.execute(&v).unwrap();
        assert_eq!(r#"1"#, r[0].to_string());
        assert_eq!(r#"1"#, r[1].to_string());
    }

    #[test]
    fn slice() {
        let q: Query = ".[2:4]".parse::<Query>().unwrap();
        let v: Value = serde_json::from_str(r#"["a","b","c","d","e"]"#).unwrap();
        assert_eq!(r#"["c","d"]"#, q.execute(&v).unwrap()[0].to_string());

        let v: Value = serde_json::from_str(r#""abcdefghi""#).unwrap();
        assert_eq!(r#""cd""#, q.execute(&v).unwrap()[0].to_string());

        let q: Query = ".[:3]".parse::<Query>().unwrap();
        let v: Value = serde_json::from_str(r#"["a","b","c","d","e"]"#).unwrap();
        assert_eq!(r#"["a","b","c"]"#, q.execute(&v).unwrap()[0].to_string());

        let q: Query = ".[-2:]".parse::<Query>().unwrap();
        assert_eq!(r#"["d","e"]"#, q.execute(&v).unwrap()[0].to_string());
    }

    #[test]
    fn split() {
        let q = ".foo,.bar".parse::<Query>().unwrap();
        let v: Value =
            serde_json::from_str(r#"{"foo": 42, "bar": "something else", "baz": true}"#).unwrap();
        let r = q.execute(&v).unwrap();
        assert_eq!(r#"42"#, r[0].to_string());
        assert_eq!(r#""something else""#, r[1].to_string());

        let q = ".user,.projects[]".parse::<Query>().unwrap();
        let v =
            serde_json::from_str(r#"{"user":"stedolan", "projects": ["jq", "wikiflow"]}"#).unwrap();
        let r = q.execute(&v).unwrap();
        assert_eq!(r#""stedolan""#, r[0].to_string());
        assert_eq!(r#""jq""#, r[1].to_string());
        assert_eq!(r#""wikiflow""#, r[2].to_string());

        //TODO: Splitting inside indexes still not supported
        //let q: Query = ".[4,2]".parse::<Query>().unwrap();
    }

    #[test]
    fn pipe() {
        let q = ".[]|.name".parse::<Query>().unwrap();
        let v: Value =
            serde_json::from_str(r#"[{"name":"JSON", "good":true}, {"name":"XML", "good":false}]"#)
                .unwrap();
        let r = q.execute(&v).unwrap();
        assert_eq!(r#""JSON""#, r[0].to_string());
        assert_eq!(r#""XML""#, r[1].to_string());
    }
}
