use nom::{character::complete::space0, IResult};

use crate::parse::ParseError;

pub(crate) fn before<'a, F, O>(mut f: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, ParseError>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, ParseError>,
{
    move |input: &'a str| {
        let (input, _) = space0(input)?;
        f(input)
    }
}

pub(crate) fn after<'a, F, O>(mut f: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, ParseError>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, ParseError>,
{
    move |input: &'a str| {
        let (input, o) = f(input)?;
        let (input, _) = space0(input)?;
        Ok((input, o))
    }
}

pub(crate) fn around<'a, F, O>(f: F) -> impl FnMut(&'a str) -> IResult<&'a str, O, ParseError>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, ParseError>,
{
    after(before(f))
}
