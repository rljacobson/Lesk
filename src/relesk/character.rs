#![allow(dead_code)]
/*!
  A Char is a `u16` of which 12 bits are used as follows:

  | &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;Value | Meaning |
  |------:|:--------|
  |    `0-255`&nbsp; | The usual ASCII character set. |
  |      `256`&nbsp; | `MIN`, a sentinel metacharacter. |
  |  `257-269`&nbsp; | The metacharacters listed in the `Meta` mod.<br>These stand for  things like "beginning of<br>line" and "end of buffer." |
  |      `270`&nbsp; | `MAX`, a sentinel metacharacter. |

  The metacharacters described here should not be confused with syntactically meaningful characters
  of a regular expression, e.g. `*`, `.`, `$`, etc., which are also sometimes called metacharacters.

  The metacharacters occupy valid Unicode code points belonging to Latin characters having
  diacritical marks, to which the metacharacters bear no relation.
*/

use std::fmt::{Formatter, Display};
use std::ops::{Add, Sub, AddAssign, SubAssign};
use std::iter::Step;


use super::*;
use std::cmp::Ordering;

// region Constants

pub const ASCII_ESCAPES : &[u8;  7] = b"abtnvfr";

/// Meta characters represent various non-characters that can match.
#[allow(non_snake_case)]
pub mod Meta {
  #![allow(non_upper_case_globals)]
  use super::*;

  pub const MIN               : Char = Char(0x100); //< Sentinel for meta characters
  pub const NonWordBoundary   : Char = Char(0x101); //< Non-word boundary at begin; `\Bx`
  pub const NonWordEnd        : Char = Char(0x102); //< Non-word boundary at end; `x\B`
  /// Beginning of word at begin; `\<x` where `\bx = (\< | \>)x`
  pub const BeginWordBegin    : Char = Char(0x103); 
  pub const EndWordBegin      : Char = Char(0x104); //< End of word at begin; `\>x`
  /// Beginning of word at end; `x\<` where `x\b= x (\< | \>)`
  pub const BeginWordEnd      : Char = Char(0x105);
  pub const EndWordEnd        : Char = Char(0x106); //< End of word at end; `x\>`
  pub const BeginningOfLine   : Char = Char(0x107); //< Beginning of line; `^`
  pub const EndOfLine         : Char = Char(0x108); //< End of line; `$`
  pub const BeginningOfBuffer : Char = Char(0x109); //< Beginning of buffer; `\A`
  pub const EndOfBuffer       : Char = Char(0x10A); //< End of buffer; `\Z`
  pub const UndentBoundary    : Char = Char(0x10B); //< Undent boundary; `\k`
  /// Indent boundary; `\i` (one less the largest META code)
  pub const IndentBoundary    : Char = Char(0x10C); 
  /// Dedent boundary; `\j` (must be the largest META code)
  pub const DedentBoundary    : Char = Char(0x10D); 
  pub const MAX               : Char = Char(0x10E); //< Sentinel for meta characters
}


pub fn meta_char_as_str(c: Char) -> &'static str {
  match c {
    Meta::NonWordBoundary   => "NWB",
    Meta::NonWordEnd        => "NWE",
    Meta::BeginWordBegin    => "BWB",
    Meta::EndWordBegin      => "EWB",
    Meta::BeginWordEnd      => "BWE",
    Meta::EndWordEnd        => "EWE",
    Meta::BeginningOfLine   => "BOL",
    Meta::EndOfLine         => "EOL",
    Meta::BeginningOfBuffer => "BOB",
    Meta::EndOfBuffer       => "EOB",
    Meta::IndentBoundary    => "IND",
    Meta::DedentBoundary    => "DED",
    Meta::UndentBoundary    => "UND",
    _                       => "",
  }
}

// endregion

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Default)]
pub struct Char(pub u16);


impl Char{

  /// Converts the escaped character (without the backslash) `c` into the character it represents.
  pub fn try_from_escape(c: Char) -> Option<Char> {
    // If it's an escape character, convert to the ASCII character it refers to.
    if let Some(index) = ASCII_ESCAPES.iter().position( | &x | Char::from(x)==c ) {
      Some(Char::from(index as u8 + '\x07' as u8)) // '\x07' is '\a'
    }
    else {
      None
    }
  }

  /**
    If `self.is_alphabetic()`, returns `self` with case inverted. Otherwise, just returns `self`.
  */
  pub fn toggle_case(&self) -> Self {
    //Assuming &self is a letter, we only need to toggle the 6th bit.
    if self.is_alphabetic() {
      Char(self.0 ^ 0b0010_0000u16)
    } else {
      *self
    }
  }

  pub fn is_alphabetic(&self) -> bool{
    !self.is_meta() && (self.0 as u8).is_ascii_alphabetic()
  }


  pub fn is_uppercase(&self) -> bool{
    !self.is_meta() && (self.0 as u8).is_ascii_uppercase()
  }


  pub fn is_lowercase(&self) -> bool{
    !self.is_meta() && (self.0 as u8).is_ascii_lowercase()
  }


  pub fn is_digit(&self) -> bool{
    !self.is_meta() && (self.0 as u8).is_ascii_digit()
  }


  pub fn is_hexdigit(&self) -> bool{
    !self.is_meta() && (self.0 as u8).is_ascii_hexdigit()
  }


  pub fn is_whitespace(&self) -> bool{
    !self.is_meta() && (self.0 as u8).is_ascii_whitespace()
  }


  pub fn is_graphic(&self) -> bool{
    !self.is_meta() && (self.0 as u8).is_ascii_graphic()
  }


  pub fn is_alphanumeric(&self) -> bool{
    !self.is_meta() && (self.0 as u8).is_ascii_alphanumeric()
  }


  pub fn to_lowercase(&self) -> Char {
    if !self.is_meta(){
      (self.0 as u8).to_ascii_lowercase().into()
    } else{
      *self
    }
  }


  pub fn to_uppercase(&self) -> Char {
    if !self.is_meta() {
      (self.0 as u8).to_ascii_uppercase().into()
    } else {
      *self
    }
  }


  pub fn to_digit(&self, radix: u32) -> u32 {
    ((self.0 as u8) as char).to_digit(radix).unwrap()
  }

  pub fn is_printable(&self) -> bool {
    const CTRL_MAX: u16 = 0x1f;

    self == '\t' || self == '\n' ||
    !(self == '\0' || (self != '\0' && self.0 <= CTRL_MAX))
  }

  /**
    This "hash" is as follows:
        limits::HASH
       =         0x1000
                     -1
       =          0xFFF
       = 0b111111111111  (twelve 1's)
                   >> 3
       =    0b111111111  (nine 1's)
    Thus, `hash()` just returns the first 9 bits of `self`.
  */
  pub fn hashed(&self) -> Hash16 {

    return self.0 & ((limits::HASH_MAX_IDX - 1) >> 3) as Hash16;
  }


  pub fn is_meta(&self) -> bool {
    // todo: Shouldn't this be >=?
    *self > Meta::MIN
  }


  pub(crate) fn escaped(&self) -> String {
    let c: u8 = self.0 as u8;

    if c >= b'\x07' && c <= b'\r' {
      // If it's an escape character, convert to the ASCII character it refers to.
      format!("\\\\{}", ASCII_ESCAPES[(c - b'\x07') as usize])
    } else if c == b'"' {
      "\\\"".to_string()
    } else if c == b'\\' {
      "\\\\".to_string()
    } else if self.is_graphic() {
      format!("{}", c as char)
    } else if c < 8 {
      format!("\\\\{}", self.0)
    } else {
      format!("\\\\x{:02x}", self.0)
    }
  }

  /// Gives a printable string representation of `self` suitable for writing to source code without
  /// consuming `self`.
  pub(crate) fn to_printable(&self) -> String {
    let c: u8 = Into::<u8>::into(*self);

    // todo: Should we handle meta characters?
    // todo: Why are the cases here different from `escape_char()`?
    if c >= b'\x07' && c <= b'\r' {
      // '\x07' is '\a'
      format!("'\\{}'", ASCII_ESCAPES[(c as u8 - b'\x07') as usize] as char)
    }
    else if c == b'\\' {
      "'\\\\'".into()
    }
    else if c == b'\'' {
      "'\\''".into()
    }
    else if self.is_printable() {
      format!("'{}'", c as char)
    }
    else {
      format!("{}", self.0)
    }
  }


}


impl Display for Char{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self.is_meta() {
      true  => write!(f, "{}", meta_char_as_str(*self)),
      false => write!(f, "{}", char::from(*self))
    }
  }
}

// region arithmetic

impl Add<u16> for Char{
  type Output = Char;

  fn add(self, rhs: u16) -> Self::Output {
    Char(self.0 + rhs)
  }
}

impl AddAssign<u16> for Char{
  fn add_assign(&mut self, rhs: u16) {
    self.0 += rhs;
  }
}


impl Sub<u16> for Char{
  type Output = Char;

  fn sub(self, rhs: u16) -> Self::Output {
    Char(self.0 - rhs)
  }
}

impl SubAssign<u16> for Char{
  fn sub_assign(&mut self, rhs: u16) {
    self.0 -= rhs;
  }
}

// endregion

// region `From` `impl`s

impl From<char> for Char {
  fn from(c: char) -> Self {
    Char::from(c as u8)
  }
}

impl From<u8> for Char {
  fn from(b: u8) -> Self {
    Char(b as u16)
  }
}

impl From<usize> for Char {
  fn from(b: usize) -> Self {
    Char(b as u16)
  }
}

impl From<Char> for char {
  fn from(c: Char) -> Self {
    (c.0 as u8) as char
  }
}

impl From<Char> for u8 {
  fn from(c: Char) -> Self {
    c.0 as u8
  }
}

impl From<Char> for u32 {
  fn from(c: Char) -> Self {
    c.0 as u32
  }
}


impl From<Char> for usize {
  fn from(c: Char) -> Self {
    c.0 as usize
  }
}



// endregion

// region comparisons


impl std::cmp::PartialEq<char> for Char {
  fn eq(&self, other: &char) -> bool {
    (self.0).eq(&(*other as u16))
  }
}

impl std::cmp::PartialOrd<char> for Char{
  fn partial_cmp(&self, other: &char) -> Option<Ordering> {
    (self.0).partial_cmp(&(*other as u16))
  }
}

impl std::cmp::PartialEq<char> for &Char{
  fn eq(&self, other: &char) -> bool {
    (self.0).eq(&(*other as u16))
  }
}


// endregion


unsafe impl Step for Char {
  fn steps_between(start: &Self, end: &Self) -> Option<usize> {

    if end.0 >= start.0 {
      return Some((end.0 - start.0) as usize);
    }
    None
  }

  fn forward_checked(start: Self, count: usize) -> Option<Self> {
    start.0.checked_add(count as u16).and_then(|x| Some(Char(x)))
  }

  fn forward(start: Self, count: usize) -> Self {
    Char(start.0 + count as u16)
  }

  fn backward_checked(start: Self, count: usize) -> Option<Self> {
    start.0.checked_sub(count as u16).and_then(|x| Some(Char(x)))
  }

  fn backward(start: Self, count: usize) -> Self {
    Char(start.0 - count as u16)
  }


  /*
  fn replace_one(&mut self) -> Self {
    std::mem::replace(self, Char(1))
  }

  fn replace_zero(&mut self) -> Self {
    std::mem::replace(self, Char(0))
  }

  fn add_one(&self) -> Self {
    let mut s = self.clone();
    s.0 += 1;
    s
  }

  fn sub_one(&self) -> Self {
    let mut s = self.clone();
    s.0 -= 1;
    s
  }

  fn add_usize(&self, n: usize) -> Option<Self> {
    self.0.checked_add(n as u16).and_then(|x| Some(Char(x)))
  }
  */
}
