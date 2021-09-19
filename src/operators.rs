use std::iter::FromIterator;

use crate::{
    null,
    parse::{parse_init, ParseError, Parseable},
    query::{iterate_results, Executable, Query},
    single, space, type_str, QueryError, QueryResult,
};
use itertools::Itertools;
use nom::{branch::alt, character::complete::char, combinator::value, IResult};
use serde_json::{Number, Value};

#[derive(Debug, PartialEq, Clone)]
pub enum Sign {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
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
        Sign::Mul => todo!(),
        Sign::Div => todo!(),
        Sign::Mod => todo!(),
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

impl Parseable for Op {
    fn parser(input: &str) -> IResult<&str, Op, ParseError> {
        let (input, left) = parse_init(input)?;
        let (input, sign) = space::around(Sign::parser)(input)?;
        let (input, right) = parse_init(input)?;
        Ok((input, Op { left, sign, right }))
    }
}

impl Parseable for Sign {
    fn parser(input: &str) -> IResult<&str, Self, ParseError> {
        alt((
            value(Sign::Add, char('+')),
            value(Sign::Sub, char('-')),
            value(Sign::Mul, char('*')),
            value(Sign::Div, char('/')),
            value(Sign::Mod, char('%')),
        ))(input)
    }
}
