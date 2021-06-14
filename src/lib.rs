use std::fmt::Debug;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Leftover characters after parsing: {0}")]
    LeftoverCharacters(String),
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

impl<E: std::fmt::Debug> From<nom::Err<E>> for ParseError {
    fn from(err: nom::Err<E>) -> Self {
        let s =match err {
            nom::Err::Incomplete(n) => format!("{:?}", n),
            nom::Err::Error(e) | nom::Err::Failure(e) => format!("{:?}", e),
        };
        ParseError::InvalidFormat(s)
    }
}

#[derive(Error, Debug)]
pub enum FilterError {
    #[error("Array index is out of bounds: {0}")]
    IndexOutOfBounds(usize),
    #[error("Object index does not exist: {0}")]
    IndexDoesNotExist(String),
    #[error("Mismatching types: expected {expected:?}, found {found:?}")]
    MismatchingTypes {
        expected: &'static str,
        found: &'static str,
    },
}

mod filter;
mod parse;
mod range;