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
    Recurse,
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
            Query::Recurse => recurse(value),
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

fn iterate_values<'a, I: IntoIterator<Item = &'a Value>>(iter: I, next: &Query) -> QueryResult {
    iterate_results(iter.into_iter().map(|vv| next.execute(vv)))
}

pub(crate) fn iterate_results<'a, I: IntoIterator<Item = QueryResult>>(iter: I) -> QueryResult {
    Ok(iter
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect())
}
