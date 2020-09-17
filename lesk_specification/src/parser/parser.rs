#![allow(dead_code, unused_imports)]

//use codespan::Span as Code;


use std::io::Read;

use nom::{
  AsChar, branch::alt, bytes::{
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
}, combinator::{
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
}, Compare, Err as NomErr, error::{
  ErrorKind,
  ParseError,
}, IResult as NomResult, multi::{
  fold_many1,
  many0,
  many1,
  separated_nonempty_list as separated_list1,
}, sequence::{
  delimited,
  pair,
  preceded,
  precededc,
  separated_pair,
  terminated,
  tuple,
}, Slice, InputLength};
use nom_locate::LocatedSpan;

use crate::{
  code::{CodeBlock, ParsedCode},
  error::{
    Error,
    Errors,
    ExpectedFoundError,
    IncorrectDelimError,
    InvalidLabelError,
    UnclosedDelimError,
    UnexpectedError
  },
  section_items::*,
};

use source::*;
use whitespace::*;
use super::*;
use crate::error::Error::{InvalidLabel, ExpectedFound, IncorrectDelim};
use nom::character::complete::line_ending;
use nom::multi::many_till;
use nom::combinator::flat_map;

// todo: make typedef for Errors
type InputType<'a> = LSpan<'a>;

// trait Parser<'a>: NomParser<InputType<'a>, InputType<'a>, Errors> {}

pub type Result< 'a>  = NomResult<InputType<'a>, InputType<'a>, Errors>;
pub type PResult<'a> = NomResult<InputType<'a>, ParsedCode, Errors>;
pub type UResult<'a> = NomResult<InputType<'a>, (), Errors>;
pub type SResult<'a> = NomResult<InputType<'a>, SectionOneItemSet, Errors>;


/// Parses section 1 of the scanner specification file.
pub fn section_1(i: InputType) -> SResult {
  // Each alternative returns a `SectionOneItemSet` which are folded into each other.

  fold_many1(

    alt((
      code,
      parse_include,
      /*state, option,*/
      value(SectionOneItemSet::default(), skip1)
    )),

    SectionOneItemSet::default(),

    | mut acc, mut next | {
      acc.append(&mut next);
      acc
    }

  )(i)
}


/**
Parses line(s) of code as found in `%{ %}`, `%%top`, `%%class`, and `%%init`, returning a
`SectionOneItemSet`. Assumes the opening delimiter has been parsed.
*/
pub fn code(i: InputType) -> SResult
{
  alt((
    // todo: Where does code within a nested `%{.. %}` go?

    // %top{
    map(
      preceded(tag("%top{"),
        |input| nested_code(input, 0, 1, false)
      ),
      |mut parsed| {
        std::mem::swap(&mut parsed.unknown_code, &mut parsed.top_code);
        parsed.into()
      },
    ),

    // %class{
    map(
      preceded(tag("%class{"),
        |input| nested_code(input, 0, 1, false)
      ),
      |mut parsed| {
        std::mem::swap(&mut parsed.unknown_code, &mut parsed.class_code);
        parsed.into()
      },
    ),

    // %init{
    map(
      preceded(tag("%init{"),
        |input| nested_code(input, 0, 1, false)
      ),
      |mut parsed| {
        std::mem::swap(&mut parsed.unknown_code, &mut parsed.init_code);
        parsed.into()
      },
    ),

    // An unknown labeled code block is an error: `%anythingelse{`

    map_res(  //::<_, _, _, _, E2: Errors, _, _, _>(
      delimited(char1('%'), alphanumeric1, char1('{')),
      |l_span: LSpan| {
        let name = l_span.slice(1..l_span.fragment().len() - 2);
        let rest = Some(i.slice((l_span.fragment().len() + 2)..));

        Err(Errors::from(
          InvalidLabel(
            InvalidLabelError::new(name, name, rest).into()
          )
        ))
      }
    ),

    // Unlabeled block code within `%{ %}`
    map(preceded(tag("%{"),
        |input| nested_code(input, 0, 1, false)
      ),
      |x| x.into()
    ),


    // ordinary user code within plain `{ }`
    map(preceded(tag("{"),
        |input| nested_code(input, 1, 0, true)
      ),
      |x| x.into()
    ),
  ))(i)
}


/**
Recursively parses blocks of code assuming the opening brace has already been consumed. The code
is accumulated in the `user_code` and/or `unknown` fields of a `ParsedCode` struct. The client
code must recategorize the `unknown` code according to which labeled block this functon was
called to parse. This is easily done with a `swap`:

    std::mem::swap(&mut code.unknown, &mut code.top_code);

This function halts parsing on error.
*/
// todo: Continue parsing after errors.
// todo: Parse string literals.
// todo: Do we have a use for `brace_level` or `block_level`
pub fn nested_code(i: InputType, mut brace_level: i32, mut block_level: i32, is_user_code: bool)
                   -> PResult
{
  if brace_level + block_level != 1 {
    // This is not the outer-most level
    map(
      many1(
        |input| parse_section_item(input, brace_level, block_level, is_user_code)
        /*
      // | i | {
      //   if block_level + brace_level == 1 {
      //     // Initial value for fold
      //     Ok((i, ParsedCode::default()))
      //   } else {
      //     Err(NomErr::Error(Errors::from(Error::Nom(i.to_span(), ErrorKind::Alt))))
      //   }
      // }

    // cond(block_level != 0 || brace_level != 0)

    //
    // // Initial value for fold
    // ParsedCode::default(),
    //
    // // Accumulate the vector of results into a single `ParsedCodeBlock` for function return value
    // |mut acc, next| {
    //   acc.append(next.into());
    //   acc
    // },

 */
      ),
      |mut v| {
        v.drain(..).fold(ParsedCode::default(), |mut acc, next| {
          acc.append(next);
          acc
        })
      }
    )(i)

  } else {
    // The outer-most level,
    parse_section_item(i, brace_level, block_level, is_user_code)
  }
}

fn parse_section_item(i: InputType, mut brace_level: i32, mut block_level: i32, is_user_code: bool) -> PResult{
  // region Local helper functions

  // Local helper function to keep DRY. (FIVE functions, four closures, three closures deep
  // inside the outer function.)
  fn make_open_delim_parser(
    open_delim: &'static str,
    brace: i32,
    block: i32,
    user_code: bool,
  ) -> impl Fn(InputType) -> PResult //NomResult<InputType, ParsedCode, Errors>
  {
    // This outer closure is here to capture `input` so we can use it in the inner closure.
    move |input| {
      preceded(
        tag(open_delim),
        |mut l_span: InputType| {
          // In the case of `%{`, exclude the `%`.
          if open_delim.starts_with("%") {
            l_span = input.slice(1..);
          }
          let mut result = nested_code(l_span, brace, block, user_code);
          match result {

            Err(NomErr::Failure(ref mut errors)) => {
              let inner_e = errors.last().unwrap();
              match inner_e {

                Error::UnclosedDelim(ref mut ude) => {
                  ude.unclosed_delims.push(input.slice(..open_delim.len()).to_span());
                  // errors.push(Error::UnclosedDelim(ude));
                  // Err(NomErr::Failure(errors))
                }

                Error::IncorrectDelim(ref mut id) => {
                  id.candidate_span = Some(input.slice(..open_delim.len()).to_span());
                  // errors.push(Error::IncorrectDelim(id));
                  // Err(NomErr::Failure(errors))
                }

                _e => { /* pass */ }

              }
            }

            ref _r => { /* pass */ }

          }
          result
        },
      )(input)
    }
  }

  // Local helper function to keep DRY.
  fn make_close_delim_parser(close_delim: &'static str, level: i32, user_code: bool)
                             -> impl Fn(InputType) -> Result
  {
    move |input| {
      preceded(tag(close_delim),
               |mut l_span: InputType| {

                 /*

                Since `level` is decremented  by virtue of returning from `nested_code`, there
                is no need to decrement it here. All that can happen is that we encounter `%}`
                when we expected `}`. Having too many closing braces manifests as an error
                elsewhere.

                However, we can detect the special case that we see too many closing braces by
                checking if level would be negative when we return if the brace is correct.

                   if !user_code {
                    let mut errors = Errors::new();
                    errors.push(Error::UnclosedDelim(
                      UnclosedDelimError::new::<LSpan, LSpan>(vec![], input)
                    ));
                    Err(NomErr::Failure(errors))
                  }
                  else
                */
                 if user_code && level == 0 {
                   // Wrong kind of closing brace
                   let mut errors = Errors::new();
                   errors.push(IncorrectDelim(
                     IncorrectDelimError::new(
                       close_delim, input.slice(..close_delim.len()), None, l_span,
                     )
                   ));
                   Err(NomErr::Failure(errors))
                 } else {
                   // In the case of `\n%}`, exclude the `%`.
                   if close_delim.starts_with("\n%") {
                     l_span = input.slice(2..);
                   }


                   Ok((input, l_span.slice(0..0)))
                 }
               },
      )(input)
    }
  }

  // Local helper function only used below.
  fn into_code_block(user_code: bool) -> impl Fn(InputType) -> ParsedCode {
    move |l_span: InputType| {
      println!("into_code_block: {}", l_span);
      if user_code {
        CodeBlock::user_code(l_span.to_span()).into()
      } else {
        CodeBlock::unknown_code(l_span.to_span()).into()
      }
    }
  }

  // endregion

  alt((
    // Only two uses of `make_open_delim_parser`
    // Opening of a new code block which has special meaning
    make_open_delim_parser("\n%{", brace_level, block_level + 1, false),
    // Opening of a new brace level
    make_open_delim_parser("{", brace_level + 1, block_level, is_user_code),

    // Only two uses of `make_close_delim_parser`
    // Closing of a code block which has special meaning
    map(
      make_close_delim_parser("\n%}", block_level, is_user_code),
      into_code_block(is_user_code),
    ),
    // Closing of a braced level
    map(
      make_close_delim_parser("}", brace_level, is_user_code),
      into_code_block(is_user_code),
    ),

    // An occurrence of `\n%%` within a block is always an error.
    preceded(
      tag("\n%%"),
      |l_span: InputType| {
        if is_user_code && block_level == 0 && brace_level == 0 {
          // We know this is a hard error, because we are outside of all blocks.
          PResult::Err(NomErr::Failure(Errors::from(ExpectedFound(
            ExpectedFoundError::new::<_, _, Span>("}", "%%", l_span.to_span()).into()
          ))))
        } else if block_level == 0 && brace_level == 0 {
          // We know this is a hard error, because we are outside of all blocks.
          PResult::Err(NomErr::Failure(Errors::from(ExpectedFound(
            ExpectedFoundError::new::<_, _, Span>("%}", "%%", l_span.to_span()).into()
          ))))
        } else {
          // This is a soft error that should just end the `many1`.
          PResult::Err(NomErr::Error(Errors::from(Error::Nom(l_span.to_span(), ErrorKind::Alt))))
        }
      },
    ),

    // Newline that doesn't match any of the above
    // map(line_ending, into_code_block(is_user_code)),

    // Comments and whitespace
    map(recognize(skip1), into_code_block(is_user_code)),

    // A string which may contain any of the special tokens above
    map(parse_string, into_code_block(is_user_code)),

    // Arbitrary line of code
    map(
      is_not("\n{}"),
      into_code_block(is_user_code),
    )
  ))(i)
}


/**
Parses a filename which is either a sequence of non-whitespace characters terminated by
whitespace or EOF or a quoted string.
*/
fn parse_filename(i: InputType) -> Result {
  alt((
    parse_string,
    preceded(not(char1('"')), is_not(" \t\r\n"))
  ))(i)
}


/**
Expression on a new line of the form:

  %include file1 "file2" "file3"

The phrase `%include` following by one or more optionally quoted file names.
*/
fn parse_include(i: InputType) -> SResult {
  let (rest, files) = preceded(
    tuple((tag("%i"), opt(tag("nclude")), is_a(" \t"))),
    cut(separated_list1(multispace1, parse_filename))
  )(i)?;

  let included_items = files.iter().map(|in_file| {
    // retrieve and parse the contents of the file
    let mut new_source = String::default();
    let mut in_file = String::from(*i.fragment());

    std::fs::File::open(&in_file)
        .expect(
          // todo: Make a proper diagnostic for this
          format!("Could not read from file: {}", &in_file).as_str()
        )
        .read_to_string(&mut new_source)
        .unwrap_or_else(
          // todo: Make a proper diagnostic for this
          |x| { panic!("Could not read from included file: {:?}", x.into_inner()); }
        );


    // todo: Figure out how to implement SourceFile, give it to codespan_reporting.

    match section_1(InputType::new(new_source.as_str())) {
      Ok((_rest, section_items)) => section_items,
      Err(errors) => {
        panic!("{}", errors);
      }
    }
  }
  ).fold(Vec::new(), |mut acc, mut next| {
    acc.append(&mut next);
    acc
  });

  Ok((rest, included_items))
}


fn parse_string(i: InputType) -> Result {
  recognize(preceded(
    char1('"'),
    cut(terminated(
      escaped(none_of(r#"\""#), '\\', one_of(r#""\/bfnrtu"#)),
      char1('"'),
    )),
  ))(i)
}


fn parse_name(i: InputType) -> NomResult<InputType, String, Errors> {
  map(
    alt((
      parse_string,
      preceded(not(char1('"')), is_not(" \t\r\n"))
    )),
    |l_span| {
      // Normalize `-` to `_`
      l_span.to_string().replace("-", "_")
    }
  )(i)
}
