use std::collections::HashMap;
use std::num::NonZeroU32;

use byte_set::ByteSet;

use crate::parser::LSpan;
use crate::{Span, SourceID};

type RuleID = usize;
type ModeID = u8;


// Every scanner has an initial mode.
const INITIAL_MODE: Mode =
  Mode{
    name: Span{
      source_id   : SourceID(NonZeroU32(1)),
      located_span: LSpan::new("INITIAL")
    },
    exclusive: false,
    // mode_id  : 0 // The only mode with id zero.
  };




#[derive(Copy, Clone, Debug, Hash)]
struct Mode<'s> {
  name          : Span<'s>,
  pub exclusive : bool,
  // mode_id       : u16
}

impl<'a> Mode<'a> {

  pub fn new(name: Span, exclusive: bool) -> Mode{
    Mode{
      name,
      exclusive,
      // mode_id
    }
  }

  pub fn name(&self) -> &str {
    self.name().fragment()
  }

  pub fn initial_mode() -> Mode {
    INITIAL_MODE
  }

}

impl Eq for Mode{}

impl PartialEq for Mode{
  fn eq(&self, other: &Self) -> bool {
    self.name.fragment() == other.name.fragment()
  }
}

#[derive(Copy, Clone, Debug, Hash)]
struct Rule<'a> {
  // todo: Should this be a mutable string?
  regex: Span<'a>,
  code : Span<'a>,
}

/**
A `Mode` is a "start condition" or "state" in the language of lex/flex.
*/
struct Modes<'a>{
  /// A ModeID is just an index into `Modes`. It is used as a proxy for the mode at that index.
  pub modes: Vec<Mode<'a>>,
  pub rules: Vec<Rule<'a>>,
  /// A mapping from a mode to the set of rules active within that mode.
  pub mode_rules: HashMap<ModeID, RuleID>,
  /// A stack of "active" modes used in parsing modes and rules. When a rule is encountered, it
  // is added to all active modes.
  pub stack: Vec<ByteSet>,
}

impl<'a> Default for Modes<'a> {
  fn default() -> Self {
    let mut modes: Vec<Mode<'a>> = vec![Mode::initial_mode()];

    Modes{
      modes,
      rules: vec![],
      mode_rules: hash_map![],
      stack: vec![],
    }
  }

}

impl<'a> Modes<'a>{
  fn new() -> Self{
    Self::default()
  }

  fn add_mode(&mut self, name: Span, exclusive: bool) -> ModeID {
    let mode_id = self.modes.len() as u8;
    let mode = Mode{
      name,
      exclusive,
    };

    // Check for uniqueness
    // Since the total number of modes will always be small, we perform a linear search.
    match self
        .modes
        .iter()
        .position(|x| x.name() == &mode.name())
    {

      Some(index) => {
        index as u8
      }

      None => {
        self.modes.push(mode);
        mode_id
      }
    }
  }

}
