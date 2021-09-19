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
        Sign::Sub => todo!(),
        Sign::Mul => todo!(),
        Sign::Div => todo!(),
        Sign::Mod => todo!(),
    }
}

fn add(l: &Value, r: &Value) -> QueryResult {
    match (l, r) {
        (Value::Number(n), Value::Number(m)) => add_numbers(n, m),
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

fn chain_collect<T, I, O>(a: &T, b: &T) -> O
where
    T: IntoIterator<Item = I> + Clone,
    O: FromIterator<I>,
{
    a.clone().into_iter().chain(b.clone().into_iter()).collect()
}

fn add_numbers(n: &Number, m: &Number) -> QueryResult {
    let num = match (n.as_i64(), m.as_i64()) {
        (Some(i), Some(j)) => Some(Number::from(i + j)),
        _ => match (n.as_f64(), m.as_f64()) {
            (Some(i), Some(j)) => Number::from_f64(i + j),
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
