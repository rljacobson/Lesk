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
  }, Slice};
use nom::character::complete::{anychar, line_ending, multispace0};
use nom::combinator::flat_map;
use nom::multi::many_till;
use nom_locate::LocatedSpan;
use source::*;
use whitespace::*;

use crate::{
  code::{CodeBlock, ParsedCode},
  error::{
    Error,
    Errors,
    ExpectedFoundError,
    IncorrectDelimError,
    InvalidLabelError,
    UnclosedDelimError,
    UnexpectedError,
    UnexpectedSectionEndError,
  },
  section_items::*,
};
use crate::error::Error::{ExpectedFound, IncorrectDelim, InvalidLabel, UnexpectedSectionEnd};

use super::*;

// todo: make typedef for Errors
type InputType<'a> = LSpan<'a>;

// trait Parser<'a>: NomParser<InputType<'a>, InputType<'a>, Errors> {}

pub type Result<'a> = NomResult<InputType<'a>, InputType<'a>, Errors>;
pub type PResult<'a> = NomResult<InputType<'a>, ParsedCode, Errors>;
pub type UResult<'a> = NomResult<InputType<'a>, (), Errors>;
pub type SResult<'a> = NomResult<InputType<'a>, SectionItemSet, Errors>;


/// Parses section 1 of the scanner specification file.
pub fn section_1(i: InputType) -> SResult {
  // Each alternative returns a `SectionOneItemSet` which are folded into each other.
  fold_many1(
    alt((
      code,
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
fn make_code_parser(item_type: ItemType) -> impl Fn(InputType) -> SResult {
  move |input| {
    match terminated(tag(item_type.open_delimiter()), multispace0)(input) {
      Ok((rest, delim_span)) => {
        nested_code(rest, item_type).map_err(
          |mut result| {
            match &mut result {
              NomErr::Failure(errors) => {
                errors.push(
                  UnclosedDelimError::new(delim, rest).into()
                );
                result
              }

              Ok((r, item_set))
              if item_type == ItemType::Unknown => {
                // This block begins with `{` which is retained.
                SectionItemSet::insert(item_set, 0, SectionItem::unknown_code(delim_span));
                (r, item_set)
              }

              _r => _r
            } // end match on nested_code(..) error result
          } // end closure mapped onto nested_code(..) error result
        ) // call to nested_code
      } // end Ok branch of match on tag(..)(input)

      _e => _e
    } // end match on tag(..)(input)
  } // end outer closure
}


/// Parser code that's common between `code`, which parses the outermost blocks of code, and
/// `nested_code`, which parses the rest.
fn parse_code_block() -> impl Fn(InputType) -> SResult {
  // todo: Where does code within a nested `{.. }` go?

  |input| {
    alt((
      // %top{
      make_code_parser(ItemType::Top),

      // %class{
      make_code_parser(ItemType::Class),

      // %init{
      make_code_parser(ItemType::Init),

      // Unlabeled block code within `%{ %}`
      make_code_parser(ItemType::User),

      // ordinary user code within plain `{ }`
      make_code_parser(ItemType::Unknown),
    ))(input)
  }
}


/**
Parses line(s) of code as found in `%{ %}`, `%%top`, `%%class`, and `%%init`, returning a
`SectionOneItemSet`. Assumes the opening delimiter has been parsed.

There is an interesting question of what to do with input that doesn't make sense but that is
allowed to be in a flex scanner spec. I think we just call it undefined behavior and let the user
deal with it.
*/
pub fn code(i: InputType) -> SResult
{
  alt((
    parse_code_block,

    // An unknown labeled code block is an error: `%anythingelse{`.
    // todo: Should we parse the block anyway?
    map_res(
      terminated(delimited(char1('%'), alphanumeric1, char1('{')), multispace0),
      |l_span: LSpan| {
        let name = l_span.slice(1..l_span.fragment().len() - 2);
        let rest = Some(i.slice((l_span.fragment().len() + 2)..));
        Err(
          NomErr::Failure(Errors::from(
            InvalidLabel(InvalidLabelError::new(name, name, rest))
          ))
        )
      },
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
// todo: Do we have a use for `brace_level` or `block_level`
pub fn nested_code(i: InputType, item_type: ItemType) -> SResult {
  let result = // The value of this giant parser
      many_till(

        // Many Section

        alt((

          // %top{
          parse_code_block,

          // A string: "This, }, is a closing brace but does not close a block."
          map(
            parse_string,
            |item_span| SectionItemSet(item_type.new(item_span))
          ),

          // A character: '}'
          map(
            parse_character,
            |item_span| SectionItemSet(item_type.new(item_span))
          ),

          // Whitespace and comments
          map(
            skip1,
            |item_span| SectionItemSet(item_type.new(item_span))
          ),


          // Match "safe" characters. This is an optimization so we don't parse a single character at
          // a time with the next parser below.
          map(
            none_of(r#"/\"'%{}"#),
            |item_span| SectionItemSet(item_type.new(item_span))
          )

              // Any character not matched above. We use more or less the code for anychar but in a way
              // that gives an ItemSet(Item(InputType)) result
              | input: InputType | {
            let mut it = input.fragment().char_indices();
            match it.next() {

              // No characters
              None => Err(NomErr::Error(Errors::from_error_kind(input, ErrorKind::Eof))),

              Some((_rest, _c)) => match it.next() {
                Some((idx, _)) => Ok((
                  input.slice(idx..),
                  SectionItemSet(item_type.new(input.slice(..idx)))
                )),

                // Just one character remaining.
                None => Ok((
                  input.slice(input.input_len()..),
                  SectionItemSet(item_type.new(input))
                )),
              },
            }
          }
        )),

        // Until Section
        alt((

          // If the wrong closing tag is found.
          map_res(tag("%}"),
                  |input| {
                    match item_type {
                      | ItemType::Top
                      | ItemType::Class
                      | ItemType::Init
                      | ItemType::User => {
                        // The ending `%}` is thrown away.
                        Ok((input, item_type.new(input.slice(0..0))))
                      }

                      ItemType::Unknown => {
                        // Always an error.
                        Err(NomErr::Failure(Errors::from(
                          ExpectedFoundError::new("}", "%}", input).into()
                        )))
                      }

                      _t => {
                        unreachable!(
                          "Impossible internal state: parsing {} inside a nested code block.",
                          _t
                        )
                      }
                    }
                  }
          ),

          // A closing brace. Make sure it matches.
          map_res(tag("}"),
                  |input| {
                    match item_type {
                      | ItemType::Top
                      | ItemType::Class
                      | ItemType::Init
                      | ItemType::User => {
                        // Always an error.
                        Err(NomErr::Failure(Errors::from(
                          ExpectedFoundError::new("%}", "}", input).into()
                        )))
                      }

                      ItemType::Unknown => {
                        // Do not throw away the `}`
                        Ok((input.slice(1..1), item_type.new(input)))
                      }

                      _t => {
                        unreachable!(
                          "Impossible internal state: parsing {} inside a nested code block.",
                          _t
                        )
                      }
                    }
                  }
          ),

          // section separator, always an error.
          preceded(
            tag("%%"),
            |input: InputType| {
              // Always an error.
              Err(NomErr::Failure(Errors::from(
                UnexpectedSectionEndError::new(Vec::<LSpan>::default(), input).into()
              )))
            }
          ),
        )) // end alt
      )(i);

  // todo: This match is only for destructuring? Or do I need other branches?
  match result {
    // (Err(e), _) => Err(e),

    (rest, (mut item_sets, mut close_delim_item)) => {
      // Consolidate parsed value
      let item_set = item_sets.iter_mut().fold(
        SectionItemSet::default(),
        |mut acc, mut next| {
          acc.merged(next)
        }
      );
      Ok((rest, merge_or_append_items(item_set, close_delim_item)))
    }

    _ => {
      unreachable!("IUnreachable internal state: `many_till`'s result always contains a result.")
    }
  }
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
  ))
}


/*
fn parse_section_item(i: InputType, mut brace_level: i32, mut block_level: i32, is_user_code: bool) -> PResult {
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

*/
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
      escaped(none_of("\"\\"), '\\', one_of("\"\\")),
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

fn merge_or_append_items(mut items: SectionItemSet, mut item: SectionItem)
                         -> SectionItemSet
{
  if items.is_empty() {
    items.push(item);
    return items;
  }

  // Unwrap always succeeds because of preceding `if`.
  let result = items.last_mut().unwrap().merged(item);

  match result {
    Merged::Yes(i) => {
      items.push(i);
    }

    Merged::No(first, second) => {
      items.push(first);
      items.push(second);
    }
  }

  items
}
