#![allow(dead_code)]
/*!

  A `Position` describes the properties and metadata of the character at a single index into the
  regex string. It holds metadata about whether the thing at that position is greedy, an anchor, an
  accepting position, how many times it can repeat, etc.

  A `Position` has a compact 64 bit representation.

*/

use std::fmt::{Display, Formatter};
use std::collections::BTreeSet;

use super::*;
use limits::MAX_INDEX;


// `PositionSet` is a `BTreeSet` because it needs to be hashable.
pub type PositionSet = BTreeSet<Position>; //< Set of `Position`'s within the regex string.

// region Constants

pub const NPOS: u64 = u64::MAX; //< Represents an empty position

// Bit Shifts
const ITER_SHIFT : u64 = 32;
const LAZY_SL    : u64 = 56;

// Bit Masks
/// The index into the regex this `Position` describes; 32 bits
const INDEX  : u64 = MAX_INDEX as u64;
/// ITER is 16 bits at bit[32:47].
const ITER   : u64 = 0xFFFF << ITER_SHIFT; //< How many times to repeat (for `*`, `+`, `{m,n}`)
const RES1   : u64 = 1u64   << 48;         //< reserved
const RES2   : u64 = 1u64   << 49;         //< reserved
const RES3   : u64 = 1u64   << 50;         //< reserved
const RES4   : u64 = 1u64   << 51;         //< reserved
const TICKED : u64 = 1u64   << 52;         //< Is lookahead ending ) in (? = X); 1 bit
const GREEDY : u64 = 1u64   << 53;         //< Force greedy quantifiers; 1 bit
const ANCHOR : u64 = 1u64   << 54;         //< Is anchor (BOW, BOL, etc.); 1 bit
const ACCEPT : u64 = 1u64   << 55;         //< Is accepting position; 1 bit
const LAZY   : u64 = 0xFF   << LAZY_SL;    //< The "lazy" byte; 8 bits
// todo: Does `LAZY` describe which group the position is "lazy" for?

// endregion

#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Debug, Hash)]
pub struct Position(pub u64);

impl Display for Position{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    let mut formatted = String::new();

    if self.is_accept() {
      formatted.push_str(&*format!(" ({})", self.accepts()));

      if self.is_lazy() {
        formatted.push_str(&*format!("?{}", self.lazy()));
      }
      if self.is_greedy() {
        formatted.push_str(&*format!("!"));
      }
    }
    else {
      formatted.push_str(&*format!(" {}", self.idx()));

      if self.is_iterable() {
        formatted.push_str(&*format!("{}.{}", self.iterations(),  self.idx()));
      }
      if self.is_lazy() {
        formatted.push_str(&*format!("?{}", self.lazy()));
      }
      if self.is_anchor() {
        formatted.push_str(&*format!("^"));
      }
      if self.is_greedy() {
        formatted.push_str(&*format!("!"));
      }
      if self.is_ticked() {
        formatted.push_str(&*format!("'"));
      }
    }

    write!(f, "{}", formatted)
  }
}


impl Position {

  pub fn new() -> Position {
    Position(NPOS)
  }

  // region Setters

  /// Returns a copy of `Position` with the `ITER` component incremented by an amount `i`.
  pub fn increment_iter(&self, i: Iteration16) -> Position {
    return Position(self.0 + ((i as u64) << ITER_SHIFT));
  }


  /// Gives a new copy of the position with the TICKED bit set/reset.
  pub fn set_ticked(&self, b: bool) -> Position {
    match b {
      true => Position(self.0 | TICKED),

      false => Position(self.0 & !TICKED)
    }
  }


  /// Set/reset GREEDY bit.
  pub fn set_greedy(&self, b: bool) -> Position {
    match b {
      true => Position(self.0 | GREEDY),

      false => Position(self.0 & !GREEDY)
    }
  }


  /// Set/reset ANCHOR bit.
  pub fn set_anchor(&self, b: bool) -> Position {
    match b {
      true => Position(self.0 | ANCHOR),

      false => Position(self.0 & !ANCHOR)
    }
  }


  /// Set/reset ACCEPT bit.
  pub fn set_accept(&self, b: bool) -> Position {
    match b {
      true => Position(self.0 | ACCEPT),

      false => Position(self.0 & !ACCEPT)
    }
  }


  // Places `lazy_value` in topmost byte position of `self`.
  pub fn set_lazy<T>(&self, lazy_value: T) -> Position
    where T: Into<u64>
  {
    return Position((self.0 & !LAZY) | ((lazy_value.into()) << LAZY_SL));
  }

  // endregion

  // region Getters

  /// Gives self with meta bits masked out. What remains is the `index` and `iter` bits, which is
  /// the first 6 bytes.
  pub fn index_with_iter(&self) -> Position {
    return Position(self.0 & (ITER | INDEX));
  }


  // The index into the regex this `Position` describes.
  pub fn idx(&self) -> Index32 {
    return (self.0 & INDEX) as Index32;
  }

// todo: Why is accepts the first 32 bits?
  /// Truncates the `u64`/`Position` to a `u32`/`Accept`.
  pub fn accepts(&self) -> GroupIndex32 {
    return (self.0 & INDEX) as GroupIndex32;
  }


  pub fn iterations(&self) -> Iteration16 {
    return ((self.0 & ITER) >> ITER_SHIFT) as Iteration16;
  }


  /// Fetches and returns the top-most byte from self.
  pub fn lazy(&self) -> Lazy8 {
    return (self.0 >> LAZY_SL) as Lazy8;
  }


  pub fn is_ticked(&self) -> bool {
    return (self.0 & TICKED) != 0;
  }


  pub fn is_greedy(&self) -> bool {
    return (self.0 & GREEDY) != 0;
  }


  pub fn is_anchor(&self) -> bool {
    return (self.0 & ANCHOR) != 0;
  }


  pub fn is_accept(&self) -> bool {
    return (self.0 & ACCEPT) != 0;
  }


  pub fn is_lazy(&self) -> bool {
    return (self.0 >> LAZY_SL) != 0;
  }


  pub fn is_iterable(&self) -> bool {
    return self.iterations() != 0;
  }

  // endregion

}

impl Default for Position{
  fn default() -> Position{
    Position(NPOS)
  }
}

/// Transforming an `Index32` into a `Position` is common.
impl From<Index32> for Position {
  fn from(val: Index32) -> Self {
    Position(val as u64)
  }
}

impl From<Position> for u64 {
  fn from(p: Position) -> Self {
    p.0
  }
}


#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn idx_and_accepts(){
    let mut position = Position(65);
    assert_eq!(position.idx(), 65);
    assert_eq!(position.idx(), position.accepts());

    // Non-interference by other attributes
    position = position.set_accept(true);
    assert_eq!(position.idx(), 65);

    position = position.set_lazy(255);
    assert_eq!(position.idx(), 65);
  }

  #[test]
  fn accept(){
    let mut position = Position(65);
    assert!(!position.is_accept());
    position = position.set_accept(true);
    assert!(position.is_accept());
  }

  #[test]
  fn anchor(){
    let mut position = Position(65);
    assert!(!position.is_anchor());
    position = position.set_anchor(true);
    assert!(position.is_anchor());
  }

  #[test]
  fn greedy(){
    let mut position = Position(65);
    assert!(!position.is_greedy());
    position = position.set_greedy(true);
    assert!(position.is_greedy());
  }

  #[test]
  fn iterable(){
    let mut position = Position(65);
    assert!(!position.is_iterable());
    position = position.increment_iter(37);
    assert!(position.is_iterable());
    assert_eq!(position.iterations(), 37);
  }

  #[test]
  fn lazy(){
    let mut position = Position(65);
    assert!(!position.is_lazy());
    position = position.set_lazy(24);
    assert!(position.is_lazy());
    assert_eq!(position.lazy(), 24);
  }

  #[test]
  fn ticked(){
    let mut position = Position(65);
    assert!(!position.is_ticked());
    position = position.set_ticked(true);
    assert!(position.is_ticked());
  }

}
