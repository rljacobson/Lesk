#![allow(dead_code)]
#![allow(non_camel_case_types)]

/*!
  Modifiers assign to each modifier value a set of positions in the regex at which that modifier
  value is active.
*/


use ranges::GenericRange;

use super::*;
use error::RegexError;



/**
  `Mode` variants indicate which value of which mode should be set/reset. The semantics of the
  capitalization are slightly different from regex syntax: a capital letter means, "turn off for
  the given range," NOT "turn off for the given range and on everywhere else."
*/
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum Mode {
  q, Q, i, I, s, S, m, M, x, X,
}

// region `impl From`'s for `Mode`

impl From<Char> for Mode {
  /// Careful, this panics if `c` cannot be converted.
  fn from(c: Char) -> Self {
    match u8::from(c) {
      b'q' => Mode::q,
      b'i' => Mode::i,
      b's' => Mode::s,
      b'm' => Mode::m,
      b'x' => Mode::x,
      b'Q' => Mode::Q,
      b'I' => Mode::I,
      b'S' => Mode::S,
      b'M' => Mode::M,
      b'X' => Mode::X,
      _    => panic!("{}", RegexError::InvalidModifier(0))
    }
  }
}

impl From<Mode> for u8{
  fn from(mode: Mode) -> Self {
    match mode {
      Mode::q => b'q',
      Mode::i => b'i',
      Mode::s => b's',
      Mode::m => b'm',
      Mode::x => b'x',
      Mode::Q => b'Q',
      Mode::I => b'I',
      Mode::S => b'S',
      Mode::M => b'M',
      Mode::X => b'X',
    }
  }
}

impl From<Mode> for bool{
  fn from(mode: Mode) -> Self {
    match mode {
      Mode::q
      | Mode::i
      | Mode::s
      | Mode::m
      | Mode::x => true,
      Mode::Q
      | Mode::I
      | Mode::S
      | Mode::M
      | Mode::X => false,
    }
  }
}

// endregion

/**
  Modifiers in this enum are those for which different parts of the regex may have different
  modes enabled. Global-only "modifiers" are not included here.

  This module assumes modes are mutually exclusive binary values.
*/
#[derive(Clone, Eq, PartialEq, Hash, Default, Debug)]
pub struct Modifiers {
  q_x_quotes         : IndexRanges, //< Enable "X" quotation of verbatim content, also `(?q:X)`
  i_case_insensitive : IndexRanges, //< Case insensitive mode, also `(?i:X)`
  s_single_line      : IndexRanges, //< Single-line mode (dotall mode), also `(?s:X)`
  m_multiline        : IndexRanges, //< Multi-line mode, also `(?m:X)`
  x_free_spacing     : IndexRanges  //< Global single line mode, probably not needed here.
}


impl Modifiers {

  /**
   Sets the mode for the given `Range`.

   Formerly `update_modified()`
  */
  pub fn set<R, M>(&mut self, into_mode: M, range: R)
    where R: Into<GenericRange<Index32>>,
          M: Into<Mode>
  {
    let mode: Mode = into_mode.into();
    // mode modifiers i, m, (enabled: s) I, M, (disabled: S)
    let ranges = self.get_from_mode_mut(mode);
    match bool::from(mode) {
      true  => {
        *ranges += range.into();
      }
      false => {
        *ranges -= range.into();
      }
    }
  }

  /**
    Reports whether `index` is a position in which modifier `mode` is active.
  */
  pub fn is_set(&self, index: Index32, mode: Mode) -> bool {
    let ranges = self.get_from_mode(mode);
    ranges.contains(&index)
  }

  /**
    Given a `Mode`, returns a mutable reference to the `Locations` field associated to that `Mode`.
  */
  pub fn get_from_mode_mut(&mut self, mode: Mode) -> &mut IndexRanges {
    match mode {
      | Mode::q
      | Mode::Q => &mut self.q_x_quotes,
      | Mode::i
      | Mode::I => &mut self.i_case_insensitive,
      | Mode::s
      | Mode::S => &mut self.s_single_line,
      | Mode::m
      | Mode::M => &mut self.m_multiline,
      | Mode::x
      | Mode::X => &mut self.x_free_spacing
    }
  }

  /**
    Given a `Mode`, returns a reference to the `Locations` field associated to that `Mode`.
  */
  pub fn get_from_mode(&self, mode: Mode) -> &IndexRanges {
    match mode {
      | Mode::q
      | Mode::Q => &self.q_x_quotes,
      | Mode::i
      | Mode::I => &self.i_case_insensitive,
      | Mode::s
      | Mode::S => &self.s_single_line,
      | Mode::m
      | Mode::M => &self.m_multiline,
      | Mode::x
      | Mode::X => &self.x_free_spacing
    }
  }

}

