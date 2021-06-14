use crate::range::Range;
use serde_json::Value;
use thiserror::Error;

type QueryResult = Result<Vec<Value>, QueryError>;

#[derive(Error, Debug)]
pub enum QueryError {
    #[error("Array index is out of bounds: {0}")]
    IndexOutOfBounds(usize),
    #[error("Object index does not exist: {0}")]
    IndexDoesNotExist(String),
    #[error("Mismatching types: expected {expected:?}, found {found:?}")]
    MismatchingTypes {
        expected: &'static str,
        found: &'static str,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum Query {
    Empty,
    Identity,
    ObjectIndex(String, bool, Box<Query>),
    ArrayIndex(isize, bool, Box<Query>),
    Slice(Range, bool, Box<Query>),
    Iterator(bool, Box<Query>),
}

impl Query {
    pub fn execute(&self, value: &Value) -> QueryResult {
        if value.is_null() {
            return null();
        }
        match self {
            Query::Empty => empty(),
            Query::Identity => single(value.clone()),
            Query::ObjectIndex(i, opt, next) => object_index(value, i, *opt, next),
            Query::ArrayIndex(i, opt, next) => array_index(value, *i, *opt, next),
            Query::Slice(r, opt, next) => slice(value, r, *opt, next),
            Query::Iterator(opt, next) => iterate(value, *opt, next),
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

fn iterate(v: &Value, opt: bool, next: &Query) -> QueryResult {
    let mut vec = Vec::new();
    match v {
        Value::Object(obj) => {
            for vv in obj.values() {
                vec.push(vv.clone());
            }
        }
        Value::Array(arr) => {
            for vv in arr {
                vec.push(vv.clone());
            }
        }
        vv if !opt => {
            return Err(QueryError::MismatchingTypes {
                expected: "Object or Array",
                found: type_string(vv),
            })
        }
        _ => return empty(),
    }

    Ok(vec
        .iter()
        .map(|vv| next.execute(vv))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect())
}

fn object_index(v: &Value, i: &str, opt: bool, next: &Query) -> QueryResult {
    if let Value::Object(obj) = v {
        if let Some(vv) = obj.get(i) {
            next.execute(vv)
        } else {
            null()
        }
    } else {
        if opt {
            empty()
        } else {
            Err(QueryError::MismatchingTypes {
                expected: "Object",
                found: type_string(v),
            })
        }
    }
}

fn array_index(v: &Value, i: isize, opt: bool, next: &Query) -> QueryResult {
    if let Value::Array(arr) = v {
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
    } else {
        if opt {
            empty()
        } else {
            Err(QueryError::MismatchingTypes {
                expected: "Array",
                found: type_string(v),
            })
        }
    }
}

fn slice(v: &Value, r: &Range, opt: bool, next: &Box<Query>) -> QueryResult {
    let vv = match v {
        Value::Array(vec) => {
            let range = r.normalize(vec.len());
            let sliced = vec[range].to_vec();
            Value::Array(sliced)
        }
        Value::String(s) => {
            let range = r.normalize(s.len());
            let sliced = s[range].to_string();
            Value::String(sliced)
        }
        vv if !opt => {
            return Err(QueryError::MismatchingTypes {
                expected: "Array or String",
                found: type_string(vv),
            })
        }
        _ => return empty(),
    };
    next.execute(&vv)
}

fn type_string(v: &Value) -> &'static str {
    match v {
        Value::Null => "Null",
        Value::Bool(_) => "Bool",
        Value::Number(_) => "Number",
        Value::String(_) => "String",
        Value::Array(_) => "Array",
        Value::Object(_) => "Object",
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
}
