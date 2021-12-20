use crate::{
    combinator::{Chain, Optional, Split},
    construction::Construct,
    empty,
    index::Index,
    operators::Op,
    raw::Raw,
    single, type_str, QueryError, QueryResult,
};
use serde_json::Value;

#[derive(Debug, PartialEq, Clone)]
pub enum Query {
    Empty,
    Identity,
    Index(Index),
    Iterator,
    Recurse,
    Split(Box<Split>),
    Chain(Box<Chain>),
    Contruct(Construct),
    Optional(Box<Optional>),
    Raw(Raw),
    Op(Box<Op>),
}

pub trait Executable {
    fn execute(&self, value: &Value) -> QueryResult;
}

impl Executable for Query {
    fn execute(&self, value: &Value) -> QueryResult {
        match self {
            Query::Empty => empty(),
            Query::Identity => single(value.clone()),
            Query::Iterator => iterate(value),
            Query::Recurse => recurse(value),
            Query::Index(i) => i.execute(value),
            Query::Split(split) => split.execute(value),
            Query::Chain(chain) => chain.execute(value),
            Query::Contruct(c) => c.execute(value),
            Query::Optional(opt) => opt.execute(value),
            Query::Raw(r) => r.execute(value),
            Query::Op(op) => op.execute(value),
        }
    }
}

fn iterate(v: &Value) -> QueryResult {
    match v {
        Value::Array(arr) => Ok(arr.clone()),
        Value::Object(map) => Ok(map.values().into_iter().cloned().collect()),
        v => Err(QueryError::Iterate(type_str(v))),
    }
}

fn recurse(v: &Value) -> QueryResult {
    let children: Vec<_> = match v {
        Value::Array(arr) => arr.iter().collect(),
        Value::Object(map) => map.values().into_iter().collect(),
        vv => return single(vv.clone()),
    };

    let mut res = vec![v.clone()];
    res.extend(iterate_results(children.iter().map(|vv| recurse(vv)))?);
    Ok(res)
}

pub(crate) fn iterate_values<'a, I: IntoIterator<Item = &'a Value>>(
    iter: I,
    next: &Query,
) -> QueryResult {
    iterate_results(iter.into_iter().map(|vv| next.execute(vv)))
}

pub(crate) fn iterate_results<I: IntoIterator<Item = QueryResult>>(iter: I) -> QueryResult {
    Ok(iter
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect())
}
