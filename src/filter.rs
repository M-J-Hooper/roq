use crate::FilterError;
use serde_json::Value;

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