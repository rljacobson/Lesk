#![allow(dead_code, unused_imports)]

//use codespan::Span as Code;


use std::io::Read;

use nom::{AsChar, branch::alt, bytes::{
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
}, InputLength, IResult as NomResult, multi::{
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
}, Slice, InputTake};
use nom::character::complete::{anychar, line_ending, multispace0};
use nom::combinator::{flat_map, peek};
use nom::multi::many_till;
use nom_locate::LocatedSpan;
use source::*;
use whitespace::*;

use crate::{error::{
  Error,
  Errors,
  ExpectedFoundError,
  IncorrectDelimError,
  InvalidLabelError,
  UnclosedDelimError,
  UnexpectedError,
  UnexpectedSectionEndError,
}, section_items::*};
use crate::mergable::{Merged, Mergable};
use crate::error::Error::{ExpectedFound, IncorrectDelim, InvalidLabel, UnexpectedSectionEnd, UnclosedDelim};
use super::*;

// todo: make typedef for Errors
type InputType<'a> = LSpan<'a>;

// trait Parser<'a>: NomParser<InputType<'a>, InputType<'a>, Errors> {}

pub type Result<'a> = NomResult<InputType<'a>, InputType<'a>, Errors>;
pub type PResult<'a> = NomResult<InputType<'a>, Span, Errors>;
pub type IResult<'a> = NomResult<InputType<'a>, SectionItem, Errors>;
pub type SResult<'a> = NomResult<InputType<'a>, SectionItemSet, Errors>;


/// Parses section 1 of the scanner specification file.
pub fn section_1(i: InputType) -> SResult {
  // todo: change to many1_until

  // Each alternative returns a `SectionOneItemSet` which are folded into each other.
  fold_many1(
    alt((
      parse_code_block,
      parse_include,
      /*state, option,*/
      value(SectionItemSet::default(), skip1)
    )),
    SectionItemSet::default(),
    |mut acc, mut next| {
      acc.append(&mut next);
      acc
    }
  )(i)
}

/**
Constructs the parser that parses code for `ItemType` `item_type`.

There are five `ItemType`s this applies to:
```rust
ItemType::Top
ItemType::Class
ItemType::Init
ItemType::User
ItemType::Unknown
```
*/
fn make_code_parser(item_type: ItemType) -> impl Fn(InputType) -> PResult {
  move |input| {
    tag(item_type.open_delimiter())(input)
        .and_then(
          |(rest, delim_span)| {
            report(delim_span, item_type);
            nested_code(rest, item_type)
                .map_err(
                  |mut result| {
                    if let NomErr::Failure(errors) = &mut result {
                      errors.push(
                        UnclosedDelim(
                          UnclosedDelimError::new(delim_span, input.slice(delim_span.input_len()..))
                        )
                      );
                    }
                    result
                  } // end closure mapped onto nested_code(..) error result
                )
                .and_then(|(rest, inner_span)| {
                  // Put the delim_span back on the inner_span if item_type is ItemType::Unknown.
                  // println!(">>>> {}: {}", item_type, inner_span);
                  if item_type == ItemType::Unknown {
                    Ok((rest, delim_span.to_span().merge(inner_span)))
                  } else {
                    Ok((rest, inner_span))
                  }
                }
                )
          }
        )
  } // end outer closure
}


/**
Parses line(s) of code as found in `%{ %}`, `%%top`, `%%class`, and `%%init`, returning a
`SectionOneItemSet`.

There is an interesting question of what to do with input that doesn't make sense but that is
allowed to be in a flex scanner spec. I think we just call it undefined behavior and let the user
deal with it.
*/
fn parse_code_block(input: InputType) -> SResult {
  // todo: Where does code within a nested `{.. }` go?

  map(
    alt((
      // %top{
      map_res::<_, _, _, _, Errors, _, _>(
        make_code_parser(ItemType::Top),
        |span| {
          Ok(SectionItem::Top(span))
        },
      ),

      // %class{
      map_res::<_, _, _, _, Errors, _, _>(
        make_code_parser(ItemType::Class),
        |span| {
          Ok(SectionItem::Class(span))
        },
      ),

      // %init{
      map_res::<_, _, _, _, Errors, _, _>(
        make_code_parser(ItemType::Init),
        |span| {
          Ok(SectionItem::Init(span))
        },
      ),

      // Unlabeled block code within `%{ %}`
      map_res::<_, _, _, _, Errors, _, _>(
        make_code_parser(ItemType::User),
        |span| {
          Ok(SectionItem::User(span))
        },
      ),

      // ordinary code within `{ }`
      map_res::<_, _, _, _, Errors, _, _>(
        make_code_parser(ItemType::Unknown),
        |span| {
          Ok(SectionItem::Unknown(span))
        },
      ),

      // An unknown labeled code block is an error: `%anythingelse{`.
      // todo: Should we parse the block anyway?
      map_res::<_, _, _, _, Errors, _, _>(
        terminated(delimited(char1('%'), alphanumeric1, char1('{')), multispace0),
        |l_span: LSpan| {
          let name = l_span.slice(1..l_span.fragment().len() - 2);
          let rest = Some(input.slice((l_span.fragment().len() + 2)..));
          Err(
            Errors::from(
              InvalidLabel(InvalidLabelError::new(name, name, rest))
            )
          )
        },
      ),
    )),
    |item| vec![item]
  )(input)
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
// todo: Do we have a use for `brace_level` or `block_level`
pub fn nested_code<'a>(i: InputType<'a>, item_type: ItemType) -> PResult<'a> {
  map_res::<_, _, _, _, Errors, _, _>(
    many_till(

      // region Many Section

      alt((

        // {...}
        make_code_parser(ItemType::Unknown),

        // A string: "This, }, is a closing brace but does not close a block."
        map(parse_string, |l_span| { report(l_span, item_type); l_span.to_span() }),

        // A character: '}'
        map(parse_character, |l_span| { report(l_span, item_type); l_span.to_span() }),

        // Whitespace and comments
        map(recognize(skip1), |l_span| { report(l_span, item_type); l_span.to_span() }),


        // Match "safe" characters. This is an optimization so we don't parse a single character at
        // a time with the next parser below.
        map(is_not(r#"/\"'%{}"#), |l_span: InputType| { report(l_span, item_type); l_span.to_span() }),

        // Any character not matched above. We use more or less the code for anychar but in a way
        // that gives an InputType result
        |input: InputType<'a>| {
          let mut it = input.fragment().char_indices();
          match it.next() {

            // No characters
            None => Err(NomErr::Error(Errors::from_error_kind(input, ErrorKind::Eof))),

            Some((_, _)) => match it.next() {
              Some((idx, _)) => {
                let (rest, l_span) = input.take_split(idx);
                report(l_span, item_type);
                Ok((rest, l_span.to_span()))
              }

              // Just one character remaining.
              None => {
                report(input, item_type);
                Ok((
                  input.slice(input.input_len()..),
                  input.to_span()
                ))
              }
            },
          }
        }
      )),
      // endregion

      // region Until Section
      alt((

        // If the wrong closing tag is found.
        map_res(tag("%}"),
                |input: InputType| {
                  report(input, item_type);

                  if item_type == ItemType::Unknown {
                    // Always an error.
                    return
                        Err(NomErr::Failure(Errors::from(
                          ExpectedFound(ExpectedFoundError::new("}", "%}", input))
                        )));
                  } else if item_type.close_delimiter() == "%}" {
                    // The ending `%}` is thrown away.
                    Ok((input, input.slice(0..0)))
                  } else {
                    let length = input.fragment().len();
                    Ok((input.slice(length..length), input))
                  }
                }
        ),

        // A closing brace. Make sure it matches.
        map_res(tag("}"),
                |input: InputType| {
                  if item_type.close_delimiter() != "}" {
                    // Always an error.
                    Err(NomErr::Failure(Errors::from(
                      ExpectedFound(ExpectedFoundError::new("%}", "}", input))
                    )))
                  } else if item_type == ItemType::Unknown {
                    // Do not throw away the `}`
                    Ok((input.slice(1..1), input))
                  } else {
                    Ok((input.slice(1..1), input.slice(1..1)))
                  }
                }
        ),

        // section separator.
        // todo: This is an error in `nested_code`, but not in `section_1`
        map_res::<_, _, _, _, Errors, _, _>(
          peek(tag("%%")),
          |input: InputType| {
            Ok((input, input.slice(0..0)))
          }
        )

        // preceded(
        //   tag("%%"),
        //   |input: InputType| {
        //     // Always an error.
        //     Err(NomErr::Error(Errors::from(
        //       UnexpectedSectionEnd(UnexpectedSectionEndError::new(Vec::<LSpan>::default(), input))
        //     )))
        //   }
        // ),
      )) // end alt
      // endregion
    ), // end many_till

    |(mut code, (rest, mut close_delim_item))| {
      // Consolidate parsed value
      if code.is_empty() {
        return Ok(close_delim_item.to_span());
      }

      let mut code_span = code.first().unwrap().to_span();

      code_span = code[1..].iter_mut().fold(code_span,
                                            |mut acc: Span, mut next| {
                                              match acc.merged(&mut next.to_span()) {
                                                Merged::Yes(s) => { /* pass */ }
                                                Merged::No(s, _) => {
                                                  panic!("Non contiguous code: {} <--> {}", acc, next.to_span());
                                                }
                                              };
                                              acc
                                            }
      );
      code_span.merged(&mut close_delim_item.to_span());
      Ok(code_span)
    }
  )(i)
}


/**
A character. We do the easiest possible thing and parse until the next `'`, only handling the
special case of an escaped `'` or `\\`, as in `'\''`.
*/
fn parse_character(i: InputType) -> Result {
  recognize(delimited(
    tag("'"),
    escaped(none_of("'\\"), '\\', one_of("'\\")),
    tag("'")
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
      escaped(none_of(r#""\"#), '\\', one_of(r#""'/01234567U\bfnrtux"#)),
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


fn report(l_span: InputType, item_type: ItemType) {
  // println!(">>>> {}: {} at line {}, col {}, {}",
  //          item_type,
  //          l_span.fragment(),
  //          l_span.location_line(),
  //          l_span.get_column(),
  //          l_span.to_span()
  // );
}
