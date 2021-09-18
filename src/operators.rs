
use nom::{IResult, branch::alt, bytes::complete::take_while1, character::complete::{char, i32}, combinator::{map, value}, sequence::delimited};
use serde_json::Value;

use crate::{QueryResult, null, parse::{ParseError, parse_init}, query::{Query, iterate_results}, single};

#[derive(Debug, PartialEq, Clone)]
pub enum Sign {
    Add,
    Sub,
    Mul,
    Div,
    Mod
}

#[derive(Debug, PartialEq, Clone)]
pub struct Op {
    left: Query,
    sign: Sign,
    right: Query
}

impl Op {
    pub fn execute(&self, value: &Value) -> QueryResult {
       let ls = self.left.execute(value)?;
       let rs = self.right.execute(value)?;

       let mut results = Vec::new();
       for l in &ls {
           for r in &rs {
                let result = match self.sign {
                    Sign::Add => Self::add(l, r),
                    Sign::Sub => todo!(),
                    Sign::Mul => todo!(),
                    Sign::Div => todo!(),
                    Sign::Mod => todo!(),
                };
                results.push(result);
           }
       }
       iterate_results(results)
    }

    fn add(l: &Value, r: &Value) -> QueryResult {
        match (l, r) {
            (Value::Number(_), Value::Number(_)) => todo!(),
            (Value::String(_), Value::String(_)) => todo!(),
            (Value::Array(_), Value::Array(_)) => todo!(),
            (Value::Object(_), Value::Object(_)) => todo!(),
            (Value::Null, Value::Null) => null(),
            (v, Value::Null) | (Value::Null, v) => single(v.clone()),
            (v, vv) => todo!(), // Error with type_str
        }
    }
}

pub(crate) fn parse(input: &str) -> IResult<&str, Op, ParseError> {
   let (input, left) = parse_init(input)?;
   let (input, sign) = parse_sign(input)?;
   let (input, right) = parse_init(input)?;
   Ok((input, Op { left, sign, right }))
}

fn parse_sign(input: &str) -> IResult<&str, Sign, ParseError> {
    alt((
        value(Sign::Add, char('+')),
        value(Sign::Sub, char('-')),
        value(Sign::Mul, char('*')),
        value(Sign::Div, char('/')),
        value(Sign::Mod, char('%')),
    ))(input)
}