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
