/*!
  A data structure that holds the command line options. Parsing of the argument string is also
  done here.
*/

use super::*;
use crate::relesk::error::RegexError;
use std::fmt::{Display, Formatter};

/// Global modifier modes, syntax flags, and compiler options.
#[derive(Debug)]
pub struct Options {
  pub bracket_escapes     : bool,        //< disable escapes in bracket lists
  pub escape_character    : Char,        //< escape character, or > 255 for none, '\\' default
  pub filenames           : Vec<String>, //< filenames to output to.
  pub insensitive_case    : bool,        //< case insensitive mode, also `(?i:X)`
  pub multiline           : bool,        //< multi-line mode, also `(?m:X)`
  pub name                : String,      //< pattern name (for use in generated code)
  pub optimize_fsm        : bool,        //< generate optimized FSM code for option f
  pub predict_match_array : bool,        //< with option f also output predict match array for fast search with `find()`
  pub quote_with_x        : bool,        //< enable "X" quotation of verbatim content, also `(?q:X)`
  pub raise_on_error      : bool,        //< raise syntax errors
  pub single_line         : bool,        //< single-line mode (dotall mode), also `(?s:X)`
  pub write_to_stderr     : bool,        //< write error message to stderr
  pub x_freespacing       : bool,        //< free-spacing mode, also `(?x:X)`
  pub z_namespace         : String,      //< namespace (NAME1.NAME2.NAME3)
}

impl Default for Options {
  fn default() -> Self {
    Self {
      filenames: vec![],
      bracket_escapes: false,
      escape_character: '\\'.into(),
      insensitive_case: false,
      multiline: false,
      name: "".to_string(),
      optimize_fsm: false,
      predict_match_array: false,
      quote_with_x: false,
      raise_on_error: false,
      single_line: false,
      write_to_stderr: false,
      x_freespacing: false,
      z_namespace: "".to_string()
    }
  }
}

impl Display for Options{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f,
      "filenames: {}
      bracket_escapes: {}
      escape_character: {}
      insensitive_case: {}
      multiline: {}
      name: {}
      optimize_fsm: {}
      predict_match_array: {}
      quote_with_x: {}
      raise_on_error: {}
      single_line: {}
      write_to_stderr: {}
      x_freespacing: {}
      z_namespace: {}",
      self.filenames.join(", "),
      self.bracket_escapes,
      self.escape_character,
      self.insensitive_case,
      self.multiline,
      self.name,
      self.optimize_fsm,
      self.predict_match_array,
      self.quote_with_x,
      self.raise_on_error,
      self.single_line,
      self.write_to_stderr,
      self.x_freespacing,
      self.z_namespace,
    )
  }
}

impl Options {

  pub fn new(options_string: &str) -> Self {
    let mut options: Self = Self::default();

    options.parse_options(options_string);

    options
  }

  pub fn parse_options(&mut self, option_string: &str) {
    // We can index into a byte slice but not a `&str`.
    let option_bytes = option_string.as_bytes();

    // Cannot use iterator/`for` syntax, because we modify `option_index` within the loop.
    let mut option_index = 0;
    while option_index < option_bytes.len(){
      let c = option_bytes[option_index];
      match c {

        b'b' => {
          self.bracket_escapes = true;
        }
        b'i' => {
          self.insensitive_case = true;
        }
        b'm' => {
          self.multiline = true;
        }
        b'o' => {
          self.optimize_fsm = true;
        }
        b'p' => {
          self.predict_match_array = true;
        }
        b'q' => {
          self.quote_with_x = true;
        }
        b'r' => {
          self.raise_on_error = true;
        }
        b's' => {
          self.single_line = true;
        }
        b'w' => {
          self.write_to_stderr = true;
        }
        b'x' => {
          self.x_freespacing = true;
        }
        b'e' => {
          if option_index + 1 != option_bytes.len() &&
          option_bytes[option_index + 1] == b'='
          {
            option_index += 1;
          }
          self.escape_character = // the value of the following `match`
          match option_index + 1 == option_bytes.len() ||
          option_bytes[option_index + 1] == b';'
          {

            true  => {
              Char(256)
            }

            false => {
              option_index += 1;
              Char::from(option_bytes[option_index])
            }

          };
        }
        b'f' => {
          self.filenames.extend(
            parse_values(&option_bytes, &mut option_index)
          );
        }
        b'n' => {
          let mut names: Vec<String> = parse_values(&option_bytes, &mut option_index);
          match &names.len() {

            0 => {
              //Error
              // todo: Make an error for this.
            }

            1 => {
              self.name = names.pop().unwrap();
            }

            _n => {
              // Error, too many names.
              self.name = names.pop().unwrap();
            }

          } // end match on names.len()
        } // end match branch b'n'
        b'z' => {
          let mut name_spaces: Vec<String> = parse_values(&option_bytes, &mut option_index);
          match &name_spaces.len() {

            0 => {
              //Error
            }

            1 => {
              self.z_namespace = name_spaces.pop().unwrap();
            }

            _n => {
              // Error, too many names. Ignoring extras.
              self.z_namespace = name_spaces.pop().unwrap();
            }

          }
        }
        _option => {
          RegexError::UnknownOption(option_index as Index32).emit();
        }
      } // end match option_bytes[option_index]

    option_index += 1;
    } //end loop
  }

}

/**
  Parse the value associated with an option of the form `x=value`. Note that value can be a
  list. This function advances `start_index`.

  As a regex, values have the form
  ```
    =?\s*([^,;\s]+\s*[, ])*\s*([^,;\s]+\s*(;|\Z))
  ```
  where `\Z` is end of buffer. For example:
  ```
    f=one.h, one.cpp, two.cpp, stdout;
  ```
*/
fn parse_values(opt_bytes: &[u8], start_index: &mut usize) -> Vec<String> {
  let mut values: Vec<String> = Vec::new();

  if opt_bytes[*start_index + 1] == b'=' {
    *start_index += 1;
  }
  let mut end_index: usize = *start_index;

  // todo: This function is a little convoluted. It needs a rewrite.
  // Sets start_index to one before first char of value after '=', then increments `end_index` until
  // `end_index` is one past the end of value.
  while *start_index < opt_bytes.len() && opt_bytes[*start_index] != b';' {

    if end_index == opt_bytes.len()
    || opt_bytes[end_index] == b','
    || opt_bytes[end_index] == b';'
    || Char::from(opt_bytes[end_index]).is_whitespace()
    {
      // Note the condition does not hold on whitespace.
      if end_index > *start_index + 1 {
        values.push(
          String::from_utf8(
            opt_bytes[(*start_index + 1)..end_index].into()
          ).unwrap()
        );
        // We don't break here, because there may be a list of values, and we want to
        // accumulate them all.
      }
      *start_index = end_index;
    }
    end_index += 1;
  }

  //*start_index -= 1;

  values
}


#[cfg(test)]
mod test{
  use super::*;

  #[test]
  fn default_options(){
    let opt = Options::default();
    assert!(!opt.bracket_escapes);
    assert!(!opt.insensitive_case);
    assert!(!opt.multiline);
    assert!(!opt.optimize_fsm);
    assert!(!opt.predict_match_array);
    assert!(!opt.quote_with_x);
    assert!(!opt.raise_on_error);
    assert!(!opt.single_line);
    assert!(!opt.write_to_stderr);
    assert!(!opt.x_freespacing);

    assert_eq!(opt.escape_character, '\\');
    assert_eq!(opt.name, "".to_string());
    assert_eq!(opt.z_namespace, "".to_string());

    assert!(opt.filenames.is_empty());
  }

  //#[test]
  fn binary_options(){
    let opt = Options::new("bimopqrswx");
    assert!(opt.bracket_escapes);
    assert!(opt.insensitive_case);
    assert!(opt.multiline);
    assert!(opt.optimize_fsm);
    assert!(opt.predict_match_array);
    assert!(opt.quote_with_x);
    assert!(opt.raise_on_error);
    assert!(opt.single_line);
    assert!(opt.write_to_stderr);
    assert!(opt.x_freespacing);
  }


  #[test]
  fn filenames_options(){
    let opt = Options::new("bf=one.h, one.cpp, two.cpp, stdout;mo");

    assert_eq!(opt.filenames.len(), 4);

    assert_eq!(opt.filenames[0], "one.h");
    assert_eq!(opt.filenames[1], "one.cpp");
    assert_eq!(opt.filenames[2], "two.cpp");
    assert_eq!(opt.filenames[3], "stdout");

    // Non-interference with other options.
    assert!(opt.bracket_escapes);
    assert!(opt.multiline);
    assert!(opt.optimize_fsm);

  }

}
