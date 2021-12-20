use std::iter::FromIterator;

use crate::{
    null,
    parse::{parse_init, ParseError, Parseable},
    query::{iterate_results, Executable, Query},
    single, space, type_str, QueryError, QueryResult,
};
use itertools::Itertools;
use nom::{
    branch::alt,
    character::complete::char,
    combinator::{opt, value},
    sequence::pair,
    IResult,
};
use serde_json::{Map, Number, Value};

#[derive(Debug, PartialEq, Clone)]
pub enum Sign {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl Parseable for Sign {
    fn parser(input: &str) -> IResult<&str, Self, ParseError> {
        space::around(alt((
            value(Sign::Add, char('+')),
            value(Sign::Sub, char('-')),
            value(Sign::Div, char('/')),
            value(Sign::Mod, char('%')),
            value(Sign::Mul, char('*')),
        )))(input)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Op {
    pub left: Query,
    pub sign: Sign,
    pub right: Query,
}

impl Executable for Op {
    fn execute(&self, value: &Value) -> QueryResult {
        let ls = self.left.execute(value)?;
        let rs = self.right.execute(value)?;

        iterate_results(
            ls.into_iter()
                .cartesian_product(rs)
                .map(|(l, r)| operate(&self.sign, &l, &r)),
        )
    }
}

fn operate(sign: &Sign, l: &Value, r: &Value) -> QueryResult {
    match sign {
        Sign::Add => add(l, r),
        Sign::Sub => sub(l, r),
        Sign::Mul => mul(l, r),
        Sign::Div => div(l, r),
        Sign::Mod => modulus(l, r),
    }
}

fn add(l: &Value, r: &Value) -> QueryResult {
    match (l, r) {
        (Value::Number(n), Value::Number(m)) => combine_numbers(n, m, |a, b| a + b, |a, b| a + b),
        (Value::String(s), Value::String(t)) => {
            single(Value::String(chain_collect(&s.chars(), &t.chars())))
        }
        (Value::Array(a), Value::Array(b)) => single(Value::Array(chain_collect(a, b))),
        (Value::Object(o), Value::Object(p)) => single(Value::Object(chain_collect(o, p))),
        (Value::Null, Value::Null) => null(),
        (v, Value::Null) | (Value::Null, v) => single(v.clone()),
        (v, vv) => Err(QueryError::Operation("add", type_str(v), type_str(vv))),
    }
}

fn sub(l: &Value, r: &Value) -> QueryResult {
    match (l, r) {
        (Value::Number(n), Value::Number(m)) => combine_numbers(n, m, |a, b| a - b, |a, b| a - b),
        (Value::Array(a), Value::Array(b)) => single(Value::Array(
            a.clone().into_iter().filter(|v| !b.contains(v)).collect(),
        )),
        (Value::Null, Value::Null) => null(),
        (v, Value::Null) => single(v.clone()),
        (v, vv) => Err(QueryError::Operation("subtract", type_str(v), type_str(vv))),
    }
}

fn mul(l: &Value, r: &Value) -> QueryResult {
    match (l, r) {
        (Value::Number(n), Value::Number(m)) => combine_numbers(n, m, |a, b| a * b, |a, b| a * b),
        (Value::String(s), Value::Number(n)) => {
            let i = n.as_u64().ok_or(QueryError::Numerical)? as usize;
            if i == 0 {
                null()
            } else {
                single(Value::String(
                    std::iter::repeat(s.clone()).take(i).collect(),
                ))
            }
        }
        (Value::Object(o), Value::Object(p)) => single(multiply_objects(o, p)),
        (Value::Null, Value::Null) => null(),
        (v, Value::Null) | (Value::Null, v) => single(v.clone()),
        (v, vv) => Err(QueryError::Operation("multiply", type_str(v), type_str(vv))),
    }
}

fn div(l: &Value, r: &Value) -> QueryResult {
    match (l, r) {
        (Value::Number(n), Value::Number(m)) => divide_numbers(n, m, |a, b| a / b, |a, b| a / b),
        (Value::String(s), Value::String(t)) => single(Value::Array(
            s.split(t).map(|s| Value::String(s.to_string())).collect(),
        )),
        (Value::Null, Value::Null) => null(),
        (v, Value::Null) => single(v.clone()),
        (v, vv) => Err(QueryError::Operation("divide", type_str(v), type_str(vv))),
    }
}

fn modulus(l: &Value, r: &Value) -> QueryResult {
    match (l, r) {
        (Value::Number(n), Value::Number(m)) => divide_numbers(n, m, |a, b| a % b, |a, b| a % b),
        (Value::Null, Value::Null) => null(),
        (v, Value::Null) => single(v.clone()),
        (v, vv) => Err(QueryError::Operation(
            "divide (remainder)",
            type_str(v),
            type_str(vv),
        )),
    }
}

fn chain_collect<T, I, O>(a: &T, b: &T) -> O
where
    T: IntoIterator<Item = I> + Clone,
    O: FromIterator<I>,
{
    a.clone().into_iter().chain(b.clone().into_iter()).collect()
}

fn combine_numbers<F64, I64>(n: &Number, m: &Number, i: I64, f: F64) -> QueryResult
where
    I64: Fn(i64, i64) -> i64,
    F64: Fn(f64, f64) -> f64,
{
    let num = match (n.as_i64(), m.as_i64()) {
        (Some(n), Some(m)) => Some(Number::from(i(n, m))),
        _ => match (n.as_f64(), m.as_f64()) {
            (Some(n), Some(m)) => Number::from_f64(f(n, m)),
            _ => None,
        },
    };
    single(Value::Number(num.ok_or(QueryError::Numerical)?))
}

fn divide_numbers<F64, I64>(n: &Number, m: &Number, i: I64, f: F64) -> QueryResult
where
    I64: Fn(i64, i64) -> i64,
    F64: Fn(f64, f64) -> f64,
{
    let num = match (n.as_i64(), m.as_i64()) {
        (Some(_), Some(m)) if m == 0 => None,
        (Some(n), Some(m)) if n % m == 0 => Some(Number::from(i(n, m))),
        (Some(n), Some(m)) => Number::from_f64(f(n as f64, m as f64)),
        _ => match (n.as_f64(), m.as_f64()) {
            (Some(_), Some(m)) if m == 0f64 => None,
            (Some(n), Some(m)) => Number::from_f64(f(n, m)),
            _ => None,
        },
    };
    single(Value::Number(num.ok_or(QueryError::Numerical)?))
}

fn multiply_objects(l: &Map<String, Value>, r: &Map<String, Value>) -> Value {
    let mut map = l.clone();
    for (k, v) in r.into_iter() {
        let insert = match (l.get(k), v) {
            (Some(Value::Object(o)), Value::Object(p)) => multiply_objects(o, p),
            (_, v) => v.clone(),
        };
        map.insert(k.clone(), insert);
    }
    Value::Object(map)
}

pub(crate) fn parse_add(input: &str) -> IResult<&str, Query, ParseError> {
    let (input, left) = parse_mul(input)?;
    let (input, opt) = opt(pair(
        space::around(alt((
            value(Sign::Add, char('+')),
            value(Sign::Sub, char('-')),
        ))),
        parse_add,
    ))(input)?;

    if let Some((sign, right)) = opt {
        Ok((input, Query::Op(Box::new(Op { left, sign, right }))))
    } else {
        Ok((input, left))
    }
}

pub(crate) fn parse_div(input: &str) -> IResult<&str, Query, ParseError> {
    let (input, left) = parse_init(input)?;
    let (input, opt) = opt(pair(
        space::around(alt((
            value(Sign::Div, char('/')),
            value(Sign::Mod, char('%')),
        ))),
        parse_div,
    ))(input)?;

    if let Some((sign, right)) = opt {
        Ok((input, Query::Op(Box::new(Op { left, sign, right }))))
    } else {
        Ok((input, left))
    }
}

pub(crate) fn parse_mul(input: &str) -> IResult<&str, Query, ParseError> {
    let (input, left) = parse_div(input)?;
    let (input, opt) = opt(pair(space::around(value(Sign::Mul, char('*'))), parse_mul))(input)?;

    if let Some((sign, right)) = opt {
        Ok((input, Query::Op(Box::new(Op { left, sign, right }))))
    } else {
        Ok((input, left))
    }
}
