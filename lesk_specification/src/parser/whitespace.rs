
use nom::{
  AsChar,
  branch::alt,
  bytes::{
    complete::{
      escaped,
      is_a,
      take_while,
      is_not,
      take_until,
      take_while1,
      tag,
    }
  }, character::{
    complete::{
      alphanumeric0,
      alphanumeric1,
      char as char1,
      multispace1,
      newline,
      none_of,
      not_line_ending,
      one_of,
    },
    is_alphanumeric,
  },
  combinator::{
    cond,
    cut,
    map,
    map_optc,
    map_parser,
    map_res,
    not,
    opt,
    recognize,
    value,
  },
  Compare,
  Err as NomErr,
  error::{
    ErrorKind,
    ParseError,
  },
  IResult as NomResult,
  multi::{
    fold_many1,
    many0,
    many1,
    separated_nonempty_list as separated_list1,
  },
  sequence::{
    delimited,
    pair,
    preceded,
    precededc,
    separated_pair,
    terminated,
    tuple,
  },
};
use nom_locate::LocatedSpan;

use crate::{
  error::Errors,
  section_items::*,
};

use super::*;


// todo: make typedef for Errors
type InputType<'a> = LSpan<'a>;

pub type Result<'a> = NomResult<InputType<'a>, InputType<'a>, Errors>;
pub type UResult<'a> = NomResult<InputType<'a>, (), Errors>;



// region Nom Whitespace

/// Noms surrounding whitespace, including newlines, comments, and empty strings, returning the
/// result of `inner`.
pub fn ws<'a, F, O, E>(inner: F) -> impl Fn(InputType<'a>) -> NomResult<InputType<'a>, O, E>
  where
      F: Fn(InputType<'a>) -> NomResult<InputType<'a>, O, E>,
      E: ParseError<InputType<'a>>
{
  delimited::<InputType<'a>, (), O, (), E, _, _, _>(
    skip0,
    inner,
    skip0,
  )
}

/// Noms trailing whitespace, including newlines and comments.
pub fn wst<'a, O, F, E>(inner: F) -> impl Fn(InputType<'a>) -> NomResult<InputType<'a>, O, E>
  where
      F: Fn(InputType<'a>) -> NomResult<InputType<'a>, O, E>,
      E: ParseError<InputType<'a>>,
{
  terminated(
    inner,
    skip0,
  )
}

/// Same as `wst` but does not match empty string.
pub fn wst1<'a, O, F, E>(inner: F) -> impl Fn(InputType<'a>) -> NomResult<InputType<'a>, O, E>
  where
      F: Fn(InputType<'a>) -> NomResult<InputType<'a>, O, E>,
      E: ParseError<InputType<'a>>,
{
  terminated(
    inner,
    skip1,
  )
}

/// Noms whitespace, including newlines and comments, returning `()`.
pub fn skip0<'a, E: ParseError<InputType<'a>>>(i: InputType<'a>) -> NomResult<InputType<'a>, (), E>
{
  value(
    (),
    many0(
      alt((value((), multispace1), inline_comment, eol_comment))
    ),
  )(i)
}

/// Noms whitespace, including newlines and comments.
pub fn skip1<'a, E: ParseError<InputType<'a>>>(i: InputType<'a>) -> NomResult<InputType<'a>, (), E>
{
  value(
    (),
    many1(
      alt((value((), multispace1), inline_comment, eol_comment))
    ),
  )(i)
}


/// Noms whitespace, including newlines and comments.
pub fn skip_no_nl1<'a, E: ParseError<InputType<'a>>>(i: InputType<'a>)
  -> NomResult<InputType<'a>, (), E>
{
  value(
    (),
    many1(
      alt((value((), is_a(" \t")), inline_comment, eol_comment))
    ),
  )(i)
}


/// Noms whitespace, including newlines and comments.
pub fn skip_no_nl0<'a, E: ParseError<InputType<'a>>>(i: InputType<'a>)
    -> NomResult<InputType<'a>, (), E>
{
  value(
    (),
    many0(
      alt((value((), is_a(" \t")), inline_comment, eol_comment))
    ),
  )(i)
}



// Noms eol comments, excluding newlines, returning `()`.
pub fn eol_comment<'a, E: ParseError<InputType<'a>>>(i: InputType<'a>)
                                                     -> NomResult<InputType<'a>, (), E>
{
  value(
    (), // Output is thrown away.
    pair(tag("//"), is_not("\n\r")),
  )(i)
}


// Noms block comments, excluding surrounding whitespace, returning `()`.
// todo: Parse nested comments
pub fn inline_comment<'a, E: ParseError<InputType<'a>>>(i: InputType<'a>)
                                                        -> NomResult<InputType<'a>, (), E>
{
  value(
    (),
    tuple((
      tag("/*"),
      take_until("*/"),
      tag("*/")
    )),
  )(i)
}

// endregion
