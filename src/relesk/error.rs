#![allow(dead_code)]

use super::Index32;
use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u32)]
pub enum RegexError {
  // todo: Implement error messages.

  EmptyClass(Index32),           //< class `[...]` is empty, e.g. `[a&&[b]]`
  EmptyExpression(Index32),      //< regex (sub)expression should not be empty
  ExceedsLength(Index32),        //< "Regex exceeds maximum length."
  ExceedsLimits(Index32),
  InvalidAnchor(Index32),
  InvalidBackreference(Index32),
  InvalidClass(Index32),         //< invalid character class name or code point
  InvalidClassRange(Index32),    //< invalid character class range, e.g. `[Z-A]`
  InvalidCollating(Index32),     //< invalid collating element `[[.name.]]`
  InvalidEscape(Index32),
  InvalidModifier(Index32),      //< invalid `(?ismx:)` modifier
  InvalidQuantifier(Index32),    //< invalid lazy/possessive quantifier
  InvalidRepeat(Index32),        //< invalid repeat range, e.g. `{10,1}`
  InvalidSyntax(Index32),
  MismatchedBraces(Index32),
  MismatchedBrackets(Index32),
  MismatchedParens(Index32),
  MismatchedQuotation(Index32),  //< mismatched `\Q...\E` or `"..."` quotation
  UndefinedName(Index32),        //< undefined macro name (reflex tool only)
  // todo: UnknownOption should be an InvocationError, not a regex error.
  UnknownOption(Index32)
}

impl RegexError{
  /// Constructs the message associated with this error. This is distinct from the string
  /// representation of the error, which is its name and location.
  pub fn to_message(&self) -> String {
    format!("Position {:4} Error: {}", self.idx(), &"RegexError")
  }


  /// Prints the error to `stderr` and exits.
  pub fn emit(&self) -> !{
    eprintln!("Error: {}", self);
    panic!();
  }


  /// The character position at which the error occurred.
  pub fn idx(&self) -> Index32 {
    *match self{
      | RegexError::EmptyClass(loc)
      | RegexError::EmptyExpression(loc)
      | RegexError::ExceedsLength(loc)
      | RegexError::ExceedsLimits(loc)
      | RegexError::InvalidAnchor(loc)
      | RegexError::InvalidBackreference(loc)
      | RegexError::InvalidClass(loc)
      | RegexError::InvalidClassRange(loc)
      | RegexError::InvalidCollating(loc)
      | RegexError::InvalidEscape(loc)
      | RegexError::InvalidModifier(loc)
      | RegexError::InvalidQuantifier(loc)
      | RegexError::InvalidRepeat(loc)
      | RegexError::InvalidSyntax(loc)
      | RegexError::MismatchedBraces(loc)
      | RegexError::MismatchedBrackets(loc)
      | RegexError::MismatchedParens(loc)
      | RegexError::MismatchedQuotation(loc)
      | RegexError::UndefinedName(loc)
      | RegexError::UnknownOption(loc)      => loc,
    }
  }
}

// todo: This should eventually be replaced with `#[strum(message = "Mismatched parentheses.")]`
impl Display for RegexError{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      RegexError::EmptyClass(loc)           => {
        write!(f, "{} at character {}", "Empty Class",           loc)
      }
      RegexError::EmptyExpression(loc)      => {
        write!(f, "{} at character {}", "Empty Expression",      loc)
      }
      RegexError::ExceedsLength(loc)        => {
        write!(f, "{} at character {}", "Exceeds Length",        loc)
      }
      RegexError::ExceedsLimits(loc)        => {
        write!(f, "{} at character {}", "Exceeds Limits",        loc)
      }
      RegexError::InvalidAnchor(loc)        => {
        write!(f, "{} at character {}", "Invalid Anchor",        loc)
      }
      RegexError::InvalidBackreference(loc) => {
        write!(f, "{} at character {}", "Invalid Backreference", loc)
      }
      RegexError::InvalidClass(loc)         => {
        write!(f, "{} at character {}", "Invalid Class",         loc)
      }
      RegexError::InvalidClassRange(loc)    => {
        write!(f, "{} at character {}", "Invalid Class Range",   loc)
      }
      RegexError::InvalidCollating(loc)     => {
        write!(f, "{} at character {}", "Invalid Collating",     loc)
      }
      RegexError::InvalidEscape(loc)        => {
        write!(f, "{} at character {}", "Invalid Escape",        loc)
      }
      RegexError::InvalidModifier(loc)      => {
        write!(f, "{} at character {}", "Invalid Modifier",      loc)
      }
      RegexError::InvalidQuantifier(loc)    => {
        write!(f, "{} at character {}", "Invalid Quantifier",    loc)
      }
      RegexError::InvalidRepeat(loc)        => {
        write!(f, "{} at character {}", "Invalid Repeat",        loc)
      }
      RegexError::InvalidSyntax(loc)        => {
        write!(f, "{} at character {}", "Invalid Syntax",        loc)
      }
      RegexError::MismatchedBraces(loc)     => {
        write!(f, "{} at character {}", "Mismatched Braces",     loc)
      }
      RegexError::MismatchedBrackets(loc)   => {
        write!(f, "{} at character {}", "Mismatched Brackets",   loc)
      }
      RegexError::MismatchedParens(loc)     => {
        write!(f, "{} at character {}", "Mismatched Parens",     loc)
      }
      RegexError::MismatchedQuotation(loc)  => {
        write!(f, "{} at character {}", "Mismatched Quotation",  loc)
      }
      RegexError::UndefinedName(loc)        => {
        write!(f, "{} at character {}", "Undefined Name",        loc)
      }
      RegexError::UnknownOption(loc)        => {
        write!(f, "{} at character {}", "Unknown Option",        loc)
      }
    }
  }
}
