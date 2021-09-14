use serde_json::Value;
use thiserror::Error;

mod construction;
mod index;
pub mod parse;
pub mod query;
mod range;

pub type QueryResult = Result<Vec<Value>, QueryError>;

#[derive(Error, Debug)]
pub enum QueryError {
    #[error("Cannot index {0} with {1}")]
    Index(&'static str, &'static str),
    #[error("Cannot iterate over {0}")]
    Iterate(&'static str),
    #[error("Cannot use {0} as object key")]
    ObjectKey(&'static str),
}

pub(crate) fn type_str(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

pub(crate) fn single(value: Value) -> QueryResult {
    Ok(vec![value])
}

pub(crate) fn null() -> QueryResult {
    single(Value::Null)
}

pub(crate) fn empty() -> QueryResult {
    Ok(Vec::new())
}

// Tests are taken from examples at https://stedolan.github.io/jq/manual
#[cfg(test)]
mod test {
    use serde_json::Value;
    use crate::query::Query;


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
        let q: Query = ".[2:4]".parse().unwrap();
        let v: Value = serde_json::from_str(r#"["a","b","c","d","e"]"#).unwrap();
        assert_eq!(r#"["c","d"]"#, q.execute(&v).unwrap()[0].to_string());

        let v: Value = serde_json::from_str(r#""abcdefghi""#).unwrap();
        assert_eq!(r#""cd""#, q.execute(&v).unwrap()[0].to_string());

        let q: Query = ".[:3]".parse().unwrap();
        let v: Value = serde_json::from_str(r#"["a","b","c","d","e"]"#).unwrap();
        assert_eq!(r#"["a","b","c"]"#, q.execute(&v).unwrap()[0].to_string());

        let q: Query = ".[-2:]".parse().unwrap();
        assert_eq!(r#"["d","e"]"#, q.execute(&v).unwrap()[0].to_string());
    }

    #[test]
    fn split() {
        let q: Query = ".foo,.bar".parse().unwrap();
        let v: Value =
            serde_json::from_str(r#"{"foo": 42, "bar": "something else", "baz": true}"#).unwrap();
        let r = q.execute(&v).unwrap();
        assert_eq!(r#"42"#, r[0].to_string());
        assert_eq!(r#""something else""#, r[1].to_string());

        let q: Query = ".user,.projects[]".parse().unwrap();
        let v: Value =
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
        let q: Query = ".[]|.name".parse().unwrap();
        let v: Value =
            serde_json::from_str(r#"[{"name":"JSON", "good":true}, {"name":"XML", "good":false}]"#)
                .unwrap();
        let r = q.execute(&v).unwrap();
        assert_eq!(r#""JSON""#, r[0].to_string());
        assert_eq!(r#""XML""#, r[1].to_string());
    }

    #[test]
    fn array_construction() {
        let q: Query = "[.user,.projects[]]".parse().unwrap();
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

        let q: Query = "{user,title:.titles[]}".parse().unwrap();
        let r = q.execute(&v).unwrap();
        assert_eq!(
            r#"{"title":"JQ Primer","user":"stedolan"}"#,
            r[0].to_string()
        );
        assert_eq!(r#"{"title":"More JQ","user":"stedolan"}"#, r[1].to_string());

        let q: Query = "{(.user):.titles}".parse().unwrap();
        assert_eq!(
            r#"{"stedolan":["JQ Primer","More JQ"]}"#,
            q.execute(&v).unwrap()[0].to_string()
        );
    }
}