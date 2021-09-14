use crate::query::{type_str, Index, Query, QueryError, QueryResult};
use serde_json::{Map, Value};

#[derive(Debug, PartialEq, Clone)]
pub enum Construct {
    Array(Box<Query>),
    Object(Vec<(Key, Query)>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Key {
    Simple(String),
    Query(Query),
}

impl Construct {
    pub fn execute(&self, value: &Value) -> QueryResult {
        match self {
            Construct::Array(inner) => construct_array(value, inner),
            Construct::Object(kvs) => construct_object(value, kvs),
        }
    }

    pub fn shorthand(s: String) -> (Key, Query) {
        let k = Key::Simple(s.clone());
        let q = Query::Index(Index::String(s), false, Box::new(Query::Identity));
        (k, q)
    }
}

fn construct_array(v: &Value, inner: &Query) -> QueryResult {
    Ok(vec![Value::Array(inner.execute(v)?)])
}

fn construct_object(v: &Value, kvs: &Vec<(Key, Query)>) -> QueryResult {
    let mut eval_keys = Vec::new();
    let mut eval_values = Vec::new();
    for kv in kvs {
        let keys = match &kv.0 {
            Key::Simple(s) => vec![s.clone()],
            Key::Query(inner) => {
                let mut keys = Vec::new();
                for k in inner.execute(v)? {
                    match k {
                        Value::String(s) => keys.push(s),
                        vv => return Err(QueryError::ObjectKey(type_str(&vv))),
                    }
                }
                keys
            }
        };
        let values = kv.1.execute(v)?;

        eval_keys.push(keys);
        eval_values.push(values);
    }

    // For each key-value query, there is a vector of permutations
    let mut combined_perms = Vec::new();
    for i in 0..eval_keys.len() {
        let mut single_perms = Vec::new();
        for k in &eval_keys[i] {
            for v in &eval_values[i] {
                single_perms.push((k.clone(), v.clone()));
            }
        }
        combined_perms.push(single_perms);
    }

    // Now get all combinations of these permutations
    let mut objs = Vec::new();
    let n = combined_perms.iter().fold(1, |i, v| i * v.len());
    for i in 0..n {
        let mut map = Map::new();
        let mut x = i;
        for j in 0..combined_perms.len() {
            let perms = &combined_perms[j];
            let l = perms.len();

            let k = x % l;
            let kv = &perms[k];
            map.insert(kv.0.clone(), kv.1.clone());

            x /= l;
        }
        objs.push(Value::Object(map));
    }
    Ok(objs)
}
