use crate::query::{Query, QueryResult};
use serde_json::Value;

#[derive(Debug, PartialEq, Clone)]
pub enum Construct {
    Array(Box<Query>),
}

impl Construct {
    pub fn execute(&self, value: &Value) -> QueryResult {
        match self {
            Construct::Array(inner) => construct_array(value, inner),
        }
    }
}

fn construct_array(v: &Value, inner: &Query) -> QueryResult {
    Ok(vec![Value::Array(inner.execute(v)?)])
}
