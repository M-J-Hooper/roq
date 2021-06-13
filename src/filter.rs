use crate::FilterError;
use serde_json::Value;

#[derive(Debug, PartialEq)]
pub enum Filter {
    Identity,
    ObjectIndex(String, Box<Option<Filter>>),
    ArrayIndex(usize, Box<Option<Filter>>),
    Iterator(Box<Option<Filter>>),
}

type FilterResult = Result<Value, FilterError>;

impl Filter {
    pub fn filter(&self, value: &Value) -> FilterResult {
        match self {
            Filter::Identity => Ok(value.clone()),
            Filter::ObjectIndex(i, next) => object_index(value, i, next),
            Filter::ArrayIndex(i, next) => array_index(value, *i, next),
            Filter::Iterator(next) => iterate(value, next),
        }
    }
}

fn iterate(v: &Value, next: &Option<Filter>) -> FilterResult {
    let mut vec = Vec::new();
    match v {
        Value::Object(obj) => for vv in obj.values() {
            vec.push(vv.clone());
        },
        Value::Array(arr) => for vv in arr {
            vec.push(vv.clone());
        }
        vv => return Err(FilterError::MismatchingTypes {
            expected: "Object or Array",
            found: type_string(vv),
        }),
    }

    if let Some(n) = next {
        vec = vec.iter()
            .map(|vv| n.filter(vv))
            .collect::<Result<Vec<_>, _>>()?;
    }
    Ok(Value::Array(vec))
}

fn object_index(v: &Value, i: &str, next: &Option<Filter>) -> FilterResult {
    if let Value::Object(obj) = v {
        if let Some(vv) = obj.get(i) {
            let vvv = if let Some(n) = next {
                n.filter(vv)?
            } else {
                vv.clone()
            };
            Ok(vvv)
        } else {
            Err(FilterError::IndexDoesNotExist(i.to_string()))
        }
    } else {
        Err(FilterError::MismatchingTypes {
            expected: "Object",
            found: type_string(v),
        })
    }
}

fn array_index(v: &Value, i: usize, next: &Option<Filter>) -> FilterResult {
    if let Value::Array(arr) = v {
        if let Some(vv) = arr.get(i) {
            let vvv = if let Some(n) = next {
                n.filter(vv)?
            } else {
                vv.clone()
            };
            Ok(vvv)
        } else {
            Err(FilterError::IndexOutOfBounds(i))
        }
    } else {
        Err(FilterError::MismatchingTypes {
            expected: "Array",
            found: type_string(v),
        })
    }
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
        let f: Filter = ".".parse().unwrap();
        let v: Value = serde_json::from_str(r#""Hello world!""#).unwrap();
        assert_eq!(r#""Hello world!""#, f.filter(&v).unwrap().to_string());
    }

    #[test]
    fn object_index() {
        let f: Filter = ".foo".parse().unwrap();
        let v: Value = serde_json::from_str(r#"{"foo": 42, "bar": "less interesting data"}"#).unwrap();
        assert_eq!(r#"42"#, f.filter(&v).unwrap().to_string());

        let v: Value = serde_json::from_str(r#"{"notfoo": true, "alsonotfoo": false}"#).unwrap();
        assert!(f.filter(&v).is_err());

        let v: Value = serde_json::from_str(r#"{"foo": 42}"#).unwrap();
        assert_eq!(r#"42"#, f.filter(&v).unwrap().to_string());
    }

    #[test]
    fn array_index() {
        let f: Filter = ".[0]".parse().unwrap();
        let v: Value = serde_json::from_str(r#"[{"name":"JSON", "good":true},{"name":"XML", "good":false}]"#).unwrap();
        assert_eq!(r#"{"good":true,"name":"JSON"}"#, f.filter(&v).unwrap().to_string());

        let f: Filter = ".[2]".parse().unwrap();
        assert!(f.filter(&v).is_err());

        assert!(".[-2]".parse::<Filter>().is_err()); // FIXME: Implement negative indices 
        //let v: Value = serde_json::from_str(r#"[1,2,3]"#).unwrap();
        //assert_eq!(r#"2"#, f.filter(&v).unwrap().to_string());
    }

    #[test]
    fn iterator() {
        let f: Filter = ".[]".parse().unwrap();
        let v: Value = serde_json::from_str(r#"[{"name":"JSON", "good":true}, {"name":"XML", "good":false}]"#).unwrap();
        assert_eq!(r#"[{"good":true,"name":"JSON"},{"good":false,"name":"XML"}]"#, f.filter(&v).unwrap().to_string());

        let v: Value = serde_json::from_str(r#"[]"#).unwrap();
        assert_eq!(r#"[]"#, f.filter(&v).unwrap().to_string());

        let v: Value = serde_json::from_str(r#"{"a": 1, "b": 1}"#).unwrap();
        assert_eq!(r#"[1,1]"#, f.filter(&v).unwrap().to_string());
    }
}