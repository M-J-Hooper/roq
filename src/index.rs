use crate::{
    null,
    parse::ParseError,
    range::{self, Range},
    single, type_str, QueryError, QueryResult,
};
use nom::{
    branch::alt,
    bytes::complete::take_while1,
    character::complete::{char, i32},
    combinator::map,
    sequence::delimited,
    IResult,
};
use serde_json::{Map, Value};

#[derive(Debug, PartialEq, Clone)]
pub enum Index {
    String(String),
    Integer(i32),
    Slice(Range),
}

impl Index {
    pub(crate) fn execute(&self, v: &Value) -> QueryResult {
        match (v, self) {
            (Value::String(s), Index::Slice(r)) => {
                let range = r.normalize(s.len());
                let sliced = s[range].to_string();
                single(Value::String(sliced))
            }
            (Value::Array(vec), Index::Slice(r)) => {
                let range = r.normalize(vec.len());
                single(Value::Array(vec[range].to_vec()))
            }
            (Value::Object(map), Index::String(s)) => Self::index_object(map, s),
            (Value::Array(arr), Index::Integer(i)) => Self::index_array(arr, *i),
            (v, Index::String(_)) => Err(QueryError::Index(type_str(v), "string")),
            (v, Index::Integer(_)) => Err(QueryError::Index(type_str(v), "number")),
            (v, Index::Slice(_)) => Err(QueryError::Index(type_str(v), "slice")),
        }
    }

    fn index_object(map: &Map<String, Value>, s: &str) -> QueryResult {
        if let Some(vv) = map.get(s) {
            single(vv.clone())
        } else {
            null()
        }
    }

    fn index_array(arr: &Vec<Value>, i: i32) -> QueryResult {
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
            single(vv.clone())
        } else {
            null()
        }
    }
}

pub(crate) fn parse(input: &str) -> IResult<&str, Index, ParseError> {
    delimited(
        char('['),
        alt((
            map(range::parse, Index::Slice),
            map(i32, Index::Integer),
            map(
                delimited(char('"'), take_while1(|c| c != '"'), char('"')),
                |s: &str| Index::String(s.to_string()),
            ),
        )),
        char(']'),
    )(input)
}
