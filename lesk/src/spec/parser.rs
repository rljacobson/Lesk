#![allow(dead_code)]

use nom::{
  branch::alt,
  bytes::complete::{
    is_not,
    take_until,
    take_while1,
    tag
  },
  character::complete::{
    char as char1,
    alphanumeric0,
    multispace1
  },
  combinator::{
    recognize,
    map,
    opt
  },
  multi::{
    separated_list1,
    many0
  },
  sequence::{
    delimited,
    pair,
    preceded,
    separated_pair,
    terminated,
    tuple
  },
  error::ParseError,
  Err as NomErr,
  IResult,
  combinator::value,
  character::complete::not_line_ending,
  multi::many1
};

//use codespan::Span as Code;

use super::*;

pub enum Parsed{
  UserCode(Codes),
  TopCode(Codes),
  ClassCode(Codes),
  InitCode(Codes)
}



// region Nom Whitespace

/// Noms surrounding whitespace, including newlines, comments, and empty strings, returning the
/// result of `inner`.
fn ws<'a, F: 'a, O, E: ParseError<&'a str>>(inner: F) -> impl Fn(&'a str) -> IResult<&'a str, O, E>
  where
  F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
  move |i| {
    delimited(
      skip,
      &inner,
      skip
    )(i)
  }
}

/// Noms trailing whitespace, including newlines and comments.
fn wst<'a, F: 'a, O, E: ParseError<&'a str>>(inner: F) -> impl Fn(&'a str) -> IResult<&'a str, O, E>
  where
  F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
  move |i| {
    terminated(
      &inner,
      skip
    )(i)
  }
}

/// Same as `wst` but does not match empty string.
fn wst1<'a, F: 'a, O, E: ParseError<&'a str>>(inner: F) -> impl Fn(&'a str) -> IResult<&'a str,
  O, E>
  where
  F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
  move |i| {
    terminated(
      &inner,
      skip1
    )(i)
  }
}

/// Noms whitespace, including newlines and comments, returning `()`.
pub fn skip<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, (), E>
{
  map(
    many0(
      alt((map(multispace1, |_| ()), inline_comment, eol_comment))
    ),
    |_| ()
  )(i)
}


/// Noms whitespace, including newlines and comments, returning `()`.
pub fn skip1<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, (), E>
{
  map(
    many1(
      alt((map(multispace1, |_| ()), inline_comment, eol_comment))
    ),
    |_| ()
  )(i)
}


// Noms eol comments, excluding newlines, returning `()`.
pub fn eol_comment<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, (), E>
{
  value(
    (), // Output is thrown away.
    pair(tag("//"), not_line_ending)
  )(i)
}


// Noms block comments, excluding surrounding whitespace, returning `()`.
pub fn inline_comment<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, (), E>
{
  map(
    tuple((
      tag("/*"),
      take_until("*\\"),
      tag("*\\")
    )),
    |_| () // Output is thrown away.
  )(i)
}

// endregion



/// Parses section 1 of the scanner specification file.
pub fn section_1(i: LocatedSpan) -> PResult<Code> {

  alt((code, topcode, classcode, initcode, include_, state, xstate, option, ))(i)

}

/// Parses line(s) of code as found in `%{ %}`, `%%top`, `%%class`, and `%%init`.
pub fn code<'a, E: ParseError<&'a str>>(i: LocatedSpan) -> PResult<Codes> {
  map(
    tuple((
      tag("/*"),
      take_until("*\\"),
      tag("*\\")
    )),
    |_| Codes::new() // Output is thrown away.
  )(i)
}
