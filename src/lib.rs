use thiserror::Error;

#[derive(Error, Debug)]
pub enum FormatError {

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

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
