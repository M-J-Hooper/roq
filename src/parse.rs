use crate::ParseError;
use crate::filter::Filter;
use nom::IResult;
use nom::bytes::complete::*;
use nom::branch::alt;
use nom::combinator::{eof, opt};
use nom::sequence::*;
use nom::character::complete::*;
use nom::character::{is_alphabetic, is_digit};

type ParseResult<'a> = Result<Filter, ParseError>;

impl std::str::FromStr for Filter {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse(s)
    }
}

fn parse(input: &str) -> ParseResult {
    let (leftover, filter) = parser(input.as_bytes())?;
    if !leftover.is_empty() {
        let s = std::str::from_utf8(leftover).unwrap();
        return Err(ParseError::LeftoverCharacters(s.to_string()));
    }
    filter.ok_or(ParseError::EmptyInput)
}

fn parser(input: &[u8]) -> IResult<&[u8], Option<Filter>> {
    Ok(if input.is_empty() {
        (input, None)
    } else {
        let (input, filter) = alt((
            object_index,
            array_index,
            iterator,
            identity
        ))(input)?;
        (input, Some(filter))
    })
}

fn identity(input: &[u8]) -> IResult<&[u8], Filter> {
    let (input, _) = terminated(char('.'), eof)(input)?;
    Ok((input, Filter::Identity))
}

fn iterator(input: &[u8]) -> IResult<&[u8], Filter> {
    let (input, _) = tag(".[]")(input)?;
    let (input, opt) = opt(char('?'))(input)?;
    let (input, next) = parser(input)?;
    Ok((input, Filter::Iterator(opt.is_some(), Box::new(next))))   
}

fn object_index(input: &[u8]) -> IResult<&[u8], Filter> {
    let (input, _) = char('.')(input)?;
    let (input, bytes) = alt((
        take_while1(is_alphabetic),
        delimited(tag("[\""), take_while1(is_alphabetic),  tag("\"]")) //FIXME: Escaped string
    ))(input)?;
    let (input, opt) = opt(char('?'))(input)?;
    let (input, next) = parser(input)?;

    let i = std::str::from_utf8(bytes).unwrap().to_string(); //FIXME: Handle bad utf8
    Ok((input, Filter::ObjectIndex(i, opt.is_some(), Box::new(next))))   
}

fn array_index(input: &[u8]) -> IResult<&[u8], Filter> {
    let (input, _) = char('.')(input)?;
    let (input, bytes) = delimited(
        char('['), 
        take_while1(is_digit),  
        char(']')
    )(input)?;
    let (input, opt) = opt(char('?'))(input)?;
    let (input, next) = parser(input)?;

    let i = std::str::from_utf8(bytes).unwrap().parse().unwrap(); //FIXME: Handle bad utf8
    Ok((input, Filter::ArrayIndex(i, opt.is_some(), Box::new(next))))   
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn identity() {
        assert!(parse("...").is_err());
        assert_eq!(Filter::Identity, parse(".").unwrap());
    }

    #[test]
    fn iterator() {
        assert!(parse("[]").is_err());
        assert!(parse(".[").is_err());
        assert!(parse(".]").is_err());
        assert!(parse(".[][]").is_err());

        assert_eq!(Filter::Iterator(false, Box::new(None)), parse(".[]").unwrap());
        assert_eq!(Filter::Iterator(true, Box::new(None)), parse(".[]?").unwrap());
        assert_eq!(
            Filter::Iterator(false, Box::new(Some(
                Filter::Iterator(false, Box::new(Some(
                    Filter::Iterator(false, Box::new(None))
                )))
            ))),
            parse(".[].[].[]").unwrap()
        );
    }

    #[test]
    fn object_index() {
        assert!(parse("foo").is_err());
        assert!(parse(".f$$").is_err());
        assert!(parse(".[f$$]").is_err());
        assert!(parse(".[foo]").is_err());
        assert!(parse(".[\"foo]").is_err());
        assert!(parse(".[foo\"]").is_err());

        assert_eq!(Filter::ObjectIndex("foo".to_string(), false, Box::new(None)), parse(".foo").unwrap());
        assert_eq!(Filter::ObjectIndex("foo".to_string(), true, Box::new(None)), parse(".foo?").unwrap());
        assert_eq!(Filter::ObjectIndex("foo".to_string(), false, Box::new(None)), parse(".[\"foo\"]").unwrap());
        assert_eq!(Filter::ObjectIndex("foo".to_string(), true, Box::new(None)), parse(".[\"foo\"]?").unwrap());
        assert_eq!(
            Filter::ObjectIndex("foo".to_string(), false, Box::new(Some(
                Filter::ObjectIndex("bar".to_string(), false, Box::new(Some(
                    Filter::ObjectIndex("baz".to_string(), false, Box::new(None))
                )))
            ))),
            parse(".foo.bar.baz").unwrap()
        );
    }

    #[test]
    fn array_index() {
        assert!(parse("[0]").is_err());
        assert!(parse(".[a]").is_err());
        assert!(parse(".[-1]").is_err()); // TODO: Accept negative indices

        assert_eq!(Filter::ArrayIndex(0, false, Box::new(None)), parse(".[0]").unwrap());
        assert_eq!(Filter::ArrayIndex(0, true, Box::new(None)), parse(".[0]?").unwrap());
        assert_eq!(Filter::ArrayIndex(9001, false, Box::new(None)), parse(".[9001]").unwrap());
        assert_eq!(
            Filter::ArrayIndex(5, false, Box::new(Some(
                Filter::ArrayIndex(8, false, Box::new(Some(
                    Filter::ArrayIndex(13, false, Box::new(None))
                )))
            ))),
            parse(".[5].[8].[13]").unwrap()
        );
    }
}