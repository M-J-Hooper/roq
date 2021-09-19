use serde_json::Value;

use crate::{
    empty,
    query::{iterate_results, iterate_values, Executable, Query},
    QueryResult,
};

#[derive(Debug, PartialEq, Clone)]
pub struct Split(pub Query, pub Query);

impl Executable for Split {
    fn execute(&self, value: &Value) -> QueryResult {
        iterate_results(vec![self.0.execute(value), self.1.execute(value)])
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Chain(pub Query, pub Query);

impl Executable for Chain {
    fn execute(&self, value: &Value) -> QueryResult {
        iterate_values(self.0.execute(value)?.iter(), &self.1)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Optional(pub Query);

impl Executable for Optional {
    fn execute(&self, value: &Value) -> QueryResult {
        match self.0.execute(value) {
            Ok(v) => Ok(v),
            Err(_) => empty(),
        }
    }
}
