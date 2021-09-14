use crate::{
    construction::Construct, empty, index::Index, null, single, type_str, QueryError, QueryResult,
};
use serde_json::Value;

#[derive(Debug, PartialEq, Clone)]
pub enum Query {
    Empty,
    Identity,
    Index(Index),
    Iterator,
    Split(Box<Query>, Box<Query>),
    Chain(Box<Query>, Box<Query>),
    Contruct(Construct),
    Optional(Box<Query>),
}

impl Query {
    pub fn execute(&self, value: &Value) -> QueryResult {
        if value.is_null() {
            return null();
        }
        match self {
            Query::Empty => empty(),
            Query::Identity => single(value.clone()),
            Query::Index(i) => i.execute(value),
            Query::Iterator => iterate(value),
            Query::Split(curr, next) => split(value, curr, next),
            Query::Chain(curr, next) => chain(value, curr, next),
            Query::Contruct(c) => c.execute(value),
            Query::Optional(inner) => optional(inner.execute(value)),
        }
    }
}

fn optional(r: QueryResult) -> QueryResult {
    if r.is_ok() {
        r
    } else {
        empty()
    }
}

fn chain(v: &Value, curr: &Query, next: &Query) -> QueryResult {
    iterate_values(curr.execute(v)?.iter(), next)
}

fn split(v: &Value, curr: &Query, next: &Query) -> QueryResult {
    iterate_results(vec![curr.execute(v), next.execute(v)])
}

fn iterate(v: &Value) -> QueryResult {
    match v {
        Value::Array(arr) => Ok(arr.clone()),
        Value::Object(map) => Ok(map.values().into_iter().map(|v| v.clone()).collect()),
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

        let q: Query = "[.foo?]".parse().unwrap();
        let v: Value = serde_json::from_str(r#"[1,2]"#).unwrap();
        assert_eq!(r#"[]"#, q.execute(&v).unwrap()[0].to_string());
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

    #[test]
    fn array_construction() {
        let q = "[.user,.projects[]]".parse::<Query>().unwrap();
        let v: Value =
            serde_json::from_str(r#"{"user":"stedolan", "projects": ["jq", "wikiflow"]}"#).unwrap();
        assert_eq!(
            r#"["stedolan","jq","wikiflow"]"#,
            q.execute(&v).unwrap()[0].to_string()
        );

        //TODO: Numerical operations still not supported
        //let q: Query = "[.[]|.*2]".parse().unwrap();
    }

    #[test]
    fn object_construction() {
        let v: Value =
            serde_json::from_str(r#"{"user":"stedolan","titles":["JQ Primer", "More JQ"]}"#)
                .unwrap();

        let q = "{user,title:.titles[]}".parse::<Query>().unwrap();
        let r = q.execute(&v).unwrap();
        assert_eq!(
            r#"{"title":"JQ Primer","user":"stedolan"}"#,
            r[0].to_string()
        );
        assert_eq!(r#"{"title":"More JQ","user":"stedolan"}"#, r[1].to_string());

        let q = "{(.user):.titles}".parse::<Query>().unwrap();
        assert_eq!(
            r#"{"stedolan":["JQ Primer","More JQ"]}"#,
            q.execute(&v).unwrap()[0].to_string()
        );
    }
}
