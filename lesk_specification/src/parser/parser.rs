#![allow(dead_code)]

use std::io::Read;
use std::fs::File;
use std::collections::HashMap;

#[allow(unused_imports)]
// region Use Nom
use nom::{
  character::complete::{anychar, line_ending, multispace0, crlf, space0},
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
  },
  character::{
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
    ParseError
  },
  InputLength,
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
  Slice,
  InputTake,
  InputIter,
  InputTakeAtPosition,
  combinator::{flat_map, peek},
  multi::many_till,
  character::complete::{alpha1, space1},
  multi::fold_many0
};
// endregion

use smallvec::SmallVec;

use whitespace::{
  skip0,
  skip1,
  skip_no_nl0,
  skip_no_nl1
};
use source::*;
use super::*;
use crate::{
  error::Error::{
    Unexpected,
    Missing
  },
  options::{
    OptionKind,
    OPTIONS
  },
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
  mergable::{
    Merged,
    Mergable,
    merge_or_append_items,
    merge_or_push_item,
  },
  error::Error::{
    ExpectedFound,
    IncorrectDelim,
    InvalidLabel,
    UnexpectedSectionEnd,
    UnclosedDelim
  },
  error::MissingError
};

// todo: make typedef for Errors

type InputType<'s> = Span<'s, &'s str>;

// trait Parser<'a>: NomParser<InputType<'a>, InputType<'a>, Errors> {}

pub type Result<'a>  = NomResult<InputType<'a>, InputType<'a>, Errors>;
pub type IResult<'a> = NomResult<InputType<'a>, Item<'a>, Errors>;
pub type SResult<'a> = NomResult<InputType<'a>, SectionItemSet<'a>, Errors>;

// region Section One

/// Parses section one of the scanner specification file.
pub fn section_one(i: InputType) -> SResult {
  // todo: change to many1_until

  // Each alternative returns a `SectionOneItemSet`, which are folded into each other.
  terminated(
    fold_many0(
      alt((
        parse_code_block,
        parse_include,
        parse_option,
        parse_state,
        parse_definition,

        // Separating the skip_no_nl1 and newline ensures that `parse_code_block` has an
        // opportunity to see the whitespace introducing indented code.
        terminated(value(SectionItemSet::default(), skip_no_nl1), line_ending)
      )),

      SectionItemSet::default(),

      |mut acc, mut next| {
        acc.extend(&mut next.drain(..));
        acc
      }
    ),

    opt(terminated(tag("%%"), opt(line_ending)))

  )(i)
}

/**
A named definition of a regex:
  INTEGER  [0-9]+|0x[0-9a-fA-F]+
  ID       [a-z][a-z0-9]*
*/
fn parse_definition(i: InputType) -> SResult {
  let (rest, (name, _sep, regex)) = tuple((
    parse_identifier,
    space1,
    not_line_ending
  ))(i)?;

  let result = SmallVec::from_elem(
    Item::Definition {
        name: name.into(),
        code: regex.into()
      },
    1);

  Ok((rest, result))
}

/**
Parses a state definition of the form:
  %state CODE
  %xstate COMMENT
*/
fn parse_state(i: InputType) -> SResult {

  let (rest, (exclusive, name) )=
  pair(
    alt((
      map(parse_keyword("state"), |_| false),
      map(parse_keyword("xstate"), |_| true),
    )),
    parse_identifier
  )(i)?;

  let result = SmallVec::from_elem(
    Item::State {
      is_exclusive: exclusive,
      name: name.into()
    }
    , 1);

  Ok((rest, result))

}

/**
Expression on a new line of the form:

  %option noline freespace tabs=4 graphs_file="graphs.gv"

The phrase `%include` following by one or more optionally quoted file names.
*/
fn parse_option(i: InputType) -> SResult {
  let (input, _) = terminated(parse_keyword("option"), space0)(i)?;

  map(
  many1(
    alt((
      // tabs=4 namespace="ChickenScanner"
      parse_option_with_value,
      // Boolean option (line debug) or negated option (noline nodebug)
      parse_option_boolean,
    ))
  ),
    | mut options | {
      options.drain_filter(| x | x.is_some())
             .map(| x | Item::Option(x.unwrap()))
             .collect()
    }
  )(input)
}

fn parse_option_boolean(i: InputType) -> NomResult<InputType, Option<OptionField>, Errors> {
  let (rest, (negated, key)) =
      terminated(pair(opt(tag("no")), is_not(" \t=\n")), space0)(i)?;

  match OPTIONS.get(key.fragment().to_lowercase().as_str()) {

    | Some(OptionKind::String(_))
    | Some(OptionKind::Number(_)) => {
      let span_start = key.fragment().len();
      Err(NomErr::Failure(Errors::from(
        Missing(MissingError::new(
          "value assignment",
          i.slice(span_start..span_start),
          Some("This option requires a value.")
        ))
      )))
    }

    Some(OptionKind::NegatedBool(field)) => Ok((rest, Some(field(false)) )),
    Some(OptionKind::Bool(field)) => Ok((rest, Some(field(true))  )),

    Some(OptionKind::Legacy) => {
      println!("The option {} is a legacy option. Ignoring.", key);
      Ok((rest, None))
    }

    Some(OptionKind::Unimplemented) => {
      println!("The option {} is not implemented. Ignoring.", key);
      Ok((rest, None))
    }

    None => {
      let span_end = key.fragment().len() + if negated.is_some() {
        2
      } else {
        0
      };
      Err(NomErr::Failure(Errors::from(
        Unexpected(UnexpectedError::new("unknown option", i.slice(0..span_end), None))
      )))
    }
  }
}

/// Parses expressions of the form:  tabs=4 namespace="ChickenScanner"
fn parse_option_with_value(input: InputType) -> NomResult<InputType, Option<OptionField>, Errors> {
  let (rest, (key, sep, value)) = terminated(
    tuple((
      is_not(" \t=\n\r"),
      delimited(space0, tag("="), space0),
      is_not(" \t=\n\r"),
    )),
    space0
  )(input)?;

  match OPTIONS.get(key.fragment().to_lowercase().as_str()) {
    Some(OptionKind::String(field)) => {
      let (_, v) = parse_quoted(value)?;
      Ok((rest, Some(field(v.to_string())) ))
    }

    Some(OptionKind::Number(field)) => {
      let result = value.fragment().parse::<u8>();
      if result.is_err() {
        Err(NomErr::Failure(Errors::from(
          ExpectedFound(ExpectedFoundError::new("number", "cannot parse as a number", value))
        )))
      } else {
        Ok((rest, Some(field(result.unwrap())) ))
      }
    }

    | Some(OptionKind::NegatedBool(_field))
    | Some(OptionKind::Bool(_field)) => {
      let span_start = key.fragment().len();
      let span_end = span_start + sep.fragment().len() + value.fragment().len();
      Err(NomErr::Failure(Errors::from(
        Unexpected(UnexpectedError::new(
          "assignment",
          input.slice(span_start..span_end),
          Some("This is a binary option and thus takes no value.")
        ))
      )))
    }

    Some(OptionKind::Legacy) => {
      println!("The option {} is a legacy option. Ignoring.", key);
      Ok((rest, None))
    }

    Some(OptionKind::Unimplemented) => {
      println!("The option {} is not implemented. Ignoring.", key);
      Ok((rest, None))
    }

    None => {
      Err(NomErr::Failure(Errors::from(
        Unexpected(UnexpectedError::new("unknown option", key, None))
      )))
    }
  }
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
fn parse_code_type(item_type: ItemType) -> impl Fn(InputType) -> Result {
  move |input| {
    tag(item_type.open_delimiter())(input)
        .and_then(
          |(rest, delim_span)| {
            report(delim_span, item_type);
            parse_nested_code(rest, item_type)
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
                  if item_type == ItemType::Unknown {
                    Ok((rest, delim_span.into().merge(inner_span)))
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
Parses top-level line(s) of code such as `%{ %}`, `%top`, `%class`, and `%init`, returning a
`SectionOneItemSet`.

There is an interesting question of what to do with input that doesn't make sense but that is
allowed to be in a flex scanner spec. I think we just call it undefined behavior and let the user
deal with it.
*/
fn parse_code_block(i: InputType) -> SResult {
  // todo: Where does code within a nested `{.. }` go?

  map(
    alt((

      // Indented User Code
      fold_many1(
        recognize(preceded(is_a("\t "), pair(not_line_ending, line_ending))),
        SectionItemSet::default(),
        |mut acc, mut next: InputType| {
          merge_or_push_item(&mut acc, Item::User(next.into()));
          acc
        }
      ),

      // %top{
      map(
        parse_code_type(ItemType::Top),
        |span| {
          SmallVec::from_elem(Item::Top(span), 1)
        },
      ),

      // %class{
      map(
        parse_code_type(ItemType::Class),
        |span| {
          SmallVec::from_elem(Item::Class(span), 1)
        },
      ),

      // %init{
      map(
        parse_code_type(ItemType::Init),
        |span| {
          SmallVec::from_elem(Item::Init(span), 1)
        },
      ),

      // Unlabeled user code within `%{ %}`
      map(
        parse_code_type(ItemType::User),
        |span| {
          SmallVec::from_elem(Item::User(span), 1)
        },
      ),

      // ordinary code within `{ }`
      map(
        parse_code_type(ItemType::Unknown),
        |span| {
          SmallVec::from_elem(Item::Unknown(span), 1)
        },
      ),


      // Error Conditions

      // An unknown labeled code block is an error: `%anythingelse{`.
      // todo: Should we parse the block anyway?
      map_res::<_, _, _, _, Errors, _, _>(
        terminated(delimited(char1('%'), alphanumeric1, char1('{')), multispace0),
        |l_span: LSpan| {
          let name = l_span.slice(1..l_span.fragment().len() - 2);
          let rest = Some(i.slice((l_span.fragment().len() + 2)..));
          Err(
            Errors::from(
              InvalidLabel(InvalidLabelError::new(name, name, rest))
            )
          )
        },
      ),
    )),
    |item|
        item
  )(i)
}


/**
Recursively parses blocks of code assuming the opening brace has already been consumed. The code
is accumulated in the `user_code` and/or `unknown` fields of a `ParsedCode` struct. The client
code must recategorize the `unknown` code according to which labeled block this function was
called to parse.

Note that this intelligently parses nested `%{...%}` and `%keyword{...}` blocks. In the second
case, inner code is assigned the same code type as the outermost block.
*/
// todo: Continue parsing after errors.
// todo: Do we have a use for `brace_level` or `block_level`
pub fn parse_nested_code<'a>(i: InputType<'a>, item_type: ItemType) -> Result<'a> {
  map::<_, _, _, Errors, _, _>(
    many_till(

      // region Many Section

      alt((

        // {...}
        parse_code_type(ItemType::Unknown),

        // A string: "This, }, is a closing brace but does not close a block."
        map(parse_string, |l_span| {
          report(l_span, item_type);
          l_span.into()
        }),

        // A character: '}'
        map(parse_character, |l_span| {
          report(l_span, item_type);
          l_span.into()
        }),

        // Whitespace and comments
        map(recognize(skip1), |l_span| {
          report(l_span, item_type);
          l_span.into()
        }),

        // Match "safe" characters. This is an optimization so we don't parse a single character at
        // a time with the next parser below.
        map(is_not(r#"/\"'%{}"#), |l_span: InputType| {
          report(l_span, item_type);
          l_span.into()
        }),

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
                  input.into()
                ))
              }
            },
          }
        }
      )),
      // endregion

      // region Until Section
      alt((
        map_res(recognize(terminated(tag("%}"), pair(space0, line_ending))),
                |input: InputType| {
                  report(input, item_type);

                  if item_type.close_delimiter() != "%}" {
                    // If the wrong closing tag is found, it is always an error.
                    Err(NomErr::Failure(Errors::from(
                      ExpectedFound(ExpectedFoundError::new(item_type.close_delimiter(), "%}", input))
                    )))
                  } else {
                    // The ending `%}` is thrown away.
                    Ok((input.slice(2..), input.slice(0..0)))
                  }
                }
        ),

        // A closing brace. Make sure it matches.
        map_res(terminated(tag("}"), peek(opt(is_a(" \t\n")))),
                |input: InputType| {
                  if item_type.close_delimiter() != "}" {
                    // Always an error.
                    Err(NomErr::Failure(Errors::from(
                      ExpectedFound(ExpectedFoundError::new(item_type.close_delimiter(), "}", input))
                    )))
                  } else if item_type == ItemType::Unknown {
                    // Do not throw away the `}`
                    Ok((input, input))
                  } else {
                    // Throw away result
                    Ok((input.slice(1..), input.slice(0..0)))
                  }
                }
        ),

        // section separator.
        // todo: This is an error in `nested_code`, but not in the section top-level
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
      consolidate_code(code, close_delim_item)
    }
  )(i)
}



/**
Expression on a new line of the form:

  %include file1 "file2" "file3"

The phrase `%include` following by one or more optionally quoted file names.
*/
fn parse_include(i: InputType) -> SResult {
  let (rest, files) = preceded(
    parse_keyword("include"),
    cut(separated_list1(space1, parse_filename))
  )(i)?;

  for f in &files{
    println!("file: {}", f.fragment());
  }

  let included_items = files.iter().map(|in_file| {
    // retrieve and parse the contents of the file
    let mut filename = String::default();

    std::fs::File::open(in_file.fragment())
        .expect(
          // todo: Make a proper diagnostic for this
          format!("Could not read from file: {}", &in_file).as_str()
        )
        .read_to_string(&mut filename)
        .unwrap_or_else(
          // todo: Make a proper diagnostic for this
          |x| { panic!("Could not read from included file: {:?}", x.into_inner()); }
        );


    // todo: Figure out how to implement SourceFile, give it to codespan_reporting.
    let new_source = Source::new(in_file.fragment().to_string(), new_source.as_str());

    match section_one(InputType::new(
      0,
      new_source.len(),
      &new_source,
    )) {
      Ok((_rest, section_items)) => section_items,
      Err(errors) => {
        panic!("{}", errors);
      }
    }
  }
  ).fold(Vec::new(), |mut acc, mut next| {
    acc.extend(&mut next.drain(..));
    acc
  });

  Ok((rest, SmallVec::from(included_items)))
}

// endregion

// region Section Two

pub fn section_two(i: InputType) -> SResult {

  let mut items = SectionItemSet::default();

  // region Scanner Top Code

  // Section Two starts with an optional code section for code local to the scanner routine. The
  // code is either surrounded by `%{ ... %}` or is indented. We store such code in
  // `Item::ScannerTop`.
  let (_rest, mut scanner_top_code) =
    fold_many0(
      alt((

        // Indented Scanner Top Code
        fold_many1(
          recognize(preceded(is_a("\t "), pair(not_line_ending, line_ending))),
          SectionItemSet::default(),
          |mut acc, mut next: InputType| {
            merge_or_push_item(&mut acc, Item::ScannerTop(next.into()));
            acc
          }
        ),

        // Scanner Top Code within `{ }`
        map(
          parse_code_type(ItemType::ScannerTop),
          |span| {
            SmallVec::from_elem(Item::ScannerTop(span), 1)
          },
        ),

        // Scanner Top Code within `%{ ... %}`





        // Note that comments MUST be indented, as otherwise they are interpreted as regexes.

      )),

      SectionItemSet::default(),

      | mut acc, mut next | {
        acc.extend(next.drain(..));
        acc
      }
  )(i)?;

  // Since non-scanner_top_code comments from above are empty, this vector might be empty.
  if !scanner_top_code.is_empty(){
    if let Some(span) = scanner_top_code {
      items.push(Item::scanner_top_code(span));
    }
  }

  // endregion

  // Recursively parse modes (start conditions).
  let modes = Modes::default();

  // Start Conditions:
  //    <start1,start2,...>
  //    < -start1,-start2,...>

  SResult

}


// endregion


// region Generic Parsers

/**
From flex docs: A word beginning with a letter or an underscore (`_`) followed by zero or more
letters, digits, `_`, or `-` (dash). The definition is taken to begin at the first non-whitespace
character following the name and continuing to the end of the line.
*/
fn parse_identifier(i: InputType) -> Result {
  recognize(pair(
    alt((alpha1, is_a("_"))), // Name must begin with a letter or `_`.
    many0(alt((
      alphanumeric1,
      is_a("_-")
    )))
  ))(i)
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
Parses a filename, which is either a quoted string or a sequence of non-whitespace characters
terminated by whitespace or EOF.
*/
fn parse_filename(i: InputType) -> Result {
  alt((
    parse_string,
    preceded(not(char1('"')), is_not(" \t\r\n"))
  ))(i)
}


/**
Keywords of the form `%keyword` can appear as any nonempty prefix: `%k`, `%key`, etc.
*/
fn parse_keyword(keyword: &'static str) -> impl Fn(InputType) -> Result {
  move |input| {
    // If the initial `%` doesn't match, don't bother continuing.
    let mut c = input.iter_elements();
    if c.next() != Some('%') {
      return Err(NomErr::Error(Errors::from_error_kind(input, ErrorKind::Tag)));
    }

    // The first keyword char MUST match. This catches the case that `c` is exhausted after
    // the initial `%`.
    let mut k = keyword.iter_elements();
    let c_next = c.next();
    let k_next = k.next();
    if c_next.is_none() ||
        k_next.is_none() ||
        c_next.unwrap().to_lowercase().next() != k_next.unwrap().to_lowercase().next()
    {
      return Err(NomErr::Error(Errors::from_error_kind(input, ErrorKind::Tag)));
    }

    for (n, c_next) in c.enumerate() {
      if "\t ".contains(c_next) {
        // The input word is a prefix of keyword, success. Add 3 for `%`, first letter, and ws
        // character.
        return Ok(input.take_split(n + 3));
      }

      let k_next = k.next();
      // If `k_next.is_none()`, then the input has a suffix that `keyword` doesn't and hence
      // doesn't match.
      if k_next.is_none() || k_next.unwrap() != c_next {
        return Err(NomErr::Error(Errors::from_error_kind(input, ErrorKind::Tag)));
      }
    }

    // The input has been exhausted. The input word is a prefix of keyword, success.
    Ok(input.take_split(input.fragment().len()))
  }
}


/// Parses a quoted string with escapes and returns the entire string, including the surrounding
/// double quotes. No escapes are transformed.
fn parse_string(i: InputType) -> Result {
  recognize(preceded(
    char1('"'),
    cut(terminated(
      // escaped(none_of(r#""\"#), '\\', one_of(r#""'/01234567U\bfnrtux"#)),
      escaped(none_of(r#""\"#), '\\', anychar),
      char1('"'),
    )),
  ))(i)
}

/*
Parses a string of non-whitespace that is optionally surrounded by double quotes. If the
quotes are present, they are consumed but excluded from the result.

Note that `parse_string()` includes any surrounding quotes and accounts for escaped characters,
while `parse_filename()` is either `parse_string()` or a string of non-whitespace characters.
*/
fn parse_quoted(i: InputType) -> Result {
  alt((
    preceded(
      char1('"'),
      cut(terminated(
        escaped(none_of(r#""\"#), '\\', anychar),
        char1('"'),
      )),
    ),
    preceded(not(char1('"')), is_not(" \t\r\n"))
  ))(i)
}

// endregion



fn consolidate_code(mut code: Vec<InputType>, item_type: InputType) -> InputType {
  // Consolidate parsed value
  if code.is_empty() {
    return close_delim_item;
  }

  let mut first_span = code.first().unwrap().into();

  let code_span = code[1..].iter_mut().fold(
    first_span,
    |mut acc: InputType, mut next| {
      match acc.merged(&mut next) {
        Merged::Yes(_s) => { /* pass */ }
        Merged::No(_s, _) => {
          println!("Non contiguous {}: {} <--> {}", item_type, acc, next.to_span());
        }
      };
      acc
    }
  );
  code_span.merged(&mut close_delim_item.to_span());
  code_span
}



#[allow(unused_variables)]
fn report(l_span: InputType, item_type: ItemType) {
  // println!(">>>> {}: {} at line {}, col {}, {}",
  //          item_type,
  //          l_span.fragment(),
  //          l_span.location_line(),
  //          l_span.get_column(),
  //          l_span.into()
  // );
}
