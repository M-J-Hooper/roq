use crate::ParseError;
use crate::filter::Filter;
use crate::range::Range;
use nom::IResult;
use nom::bytes::complete::*;
use nom::branch::alt;
use nom::combinator::*;
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
    if input.is_empty() {
        return Ok(Filter::Empty);
    }

    let (leftover, filter) = init_parser(input.as_bytes())?;
    if !leftover.is_empty() {
        let s = std::str::from_utf8(leftover).unwrap();
        return Err(ParseError::LeftoverCharacters(s.to_string()));
    }
    Ok(filter)
}

fn init_parser(input: &[u8]) -> IResult<&[u8], Filter> {
    alt((
        object_index,
        preceded(char('.'), slice),
        preceded(char('.'), array_index),
        preceded(char('.'), iterator),
        value(Filter::Identity, preceded(char('.'), eof))
    ))(input)
}

fn parser(input: &[u8]) -> IResult<&[u8], Filter> {
    alt((
        object_index,
        slice,
        array_index,
        iterator,
        value(Filter::Identity, eof)
    ))(input)
}

fn iterator(input: &[u8]) -> IResult<&[u8], Filter> {
    let (input, _) = tag("[]")(input)?;
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
    let (input, i) = delimited(
        char('['), 
        num,  
        char(']')
    )(input)?;
    let (input, opt) = opt(char('?'))(input)?;
    let (input, next) = parser(input)?;

    Ok((input, Filter::ArrayIndex(i, opt.is_some(), Box::new(next))))   
}

fn slice(input: &[u8]) -> IResult<&[u8], Filter> {
    let (input, r) = delimited(
        char('['), 
        alt((
            map(separated_pair(num, char(':'), num), Range::new),
            map(preceded(char(':'), num), Range::upper),
            map(terminated(num, char(':')), Range::lower)
        )),  
        char(']')
    )(input)?;
    let (input, opt) = opt(char('?'))(input)?;
    let (input, next) = parser(input)?;

    Ok((input, Filter::Slice(r, opt.is_some(), Box::new(next))))
}

fn num(input: &[u8]) -> IResult<&[u8], isize> {
    let (input, neg) = opt(char('-'))(input)?;
    let (input, mut i) = map_res(
        map_res(take_while1(is_digit), std::str::from_utf8),
        std::str::FromStr::from_str
    )(input)?;

    if neg.is_some() {
        i *= -1;
    }
    Ok((input, i))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn simple() {
        assert!(parse("...").is_err());
        assert_eq!(Filter::Identity, parse(".").unwrap());
        assert_eq!(Filter::Empty, parse("").unwrap());
    }

    #[test]
    fn iterator() {
        assert!(parse("[]").is_err());
        assert!(parse(".[").is_err());
        assert!(parse(".]").is_err());
        assert!(parse(".[].[]").is_err());

        assert_eq!(Filter::Iterator(false, Box::new(Filter::Identity)), parse(".[]").unwrap());
        assert_eq!(Filter::Iterator(true, Box::new(Filter::Identity)), parse(".[]?").unwrap());
        assert_eq!(
            Filter::Iterator(false, Box::new(
                Filter::Iterator(false, Box::new(
                    Filter::Iterator(false, Box::new(Filter::Identity))
                ))
            )),
            parse(".[][][]").unwrap()
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

        assert_eq!(Filter::ObjectIndex("foo".to_string(), false, Box::new(Filter::Identity)), parse(".foo").unwrap());
        assert_eq!(Filter::ObjectIndex("foo".to_string(), true, Box::new(Filter::Identity)), parse(".foo?").unwrap());
        assert_eq!(Filter::ObjectIndex("foo".to_string(), false, Box::new(Filter::Identity)), parse(".[\"foo\"]").unwrap());
        assert_eq!(Filter::ObjectIndex("foo".to_string(), true, Box::new(Filter::Identity)), parse(".[\"foo\"]?").unwrap());
        assert_eq!(
            Filter::ObjectIndex("foo".to_string(), false, Box::new(
                Filter::ObjectIndex("bar".to_string(), false, Box::new(
                    Filter::ObjectIndex("baz".to_string(), false, Box::new(Filter::Identity))
                ))
            )),
            parse(".foo.bar.baz").unwrap()
        );
    }

    #[test]
    fn array_index() {
        assert!(parse("[0]").is_err());
        assert!(parse(".[a]").is_err());
        assert!(parse(".[0].[0]").is_err());

        assert_eq!(Filter::ArrayIndex(0, false, Box::new(Filter::Identity)), parse(".[0]").unwrap());
        assert_eq!(Filter::ArrayIndex(-1, false, Box::new(Filter::Identity)), parse(".[-1]").unwrap());
        assert_eq!(Filter::ArrayIndex(0, true, Box::new(Filter::Identity)), parse(".[0]?").unwrap());
        assert_eq!(Filter::ArrayIndex(9001, false, Box::new(Filter::Identity)), parse(".[9001]").unwrap());
        assert_eq!(
            Filter::ArrayIndex(5, false, Box::new(
                Filter::ArrayIndex(8, false, Box::new(
                    Filter::ArrayIndex(13, false, Box::new(Filter::Identity))
                ))
            )),
            parse(".[5][8][13]").unwrap()
        );
    }

    #[test]
    fn slice() {
        assert!(parse(".[:]").is_err());
        assert!(parse(".[1::2]").is_err());
        assert!(parse(".[:2:]").is_err());
        assert!(parse(".[--2]").is_err());
        assert!(parse(".[-2:4:]").is_err());
        assert!(parse(".[:-2:4]").is_err());

        assert_eq!(Filter::Slice(Range::new((-1, 2)), false, Box::new(Filter::Identity)), parse(".[-1:2]").unwrap());
        assert_eq!(Filter::Slice(Range::upper(2), false, Box::new(Filter::Identity)), parse(".[:2]").unwrap());
        assert_eq!(Filter::Slice(Range::lower(1), true, Box::new(Filter::Identity)), parse(".[1:]?").unwrap());
        assert_eq!(Filter::Slice(Range::new((9001, -9001)), false, Box::new(Filter::Identity)), parse(".[9001:-9001]").unwrap());
    }
}