#![allow(dead_code)]

/*!

  `Chars`, i.e. subsets of the set of all ASCII and meta characters, are represented compactly in a
  `Chars` struct as a bitfield of 5 `u64`'s: a character with code n is in a given class if and only
  if the nth bit is set in the class's bitfield representation.

  Some subsets are special, such as the POSIX character classes. The `CharClass` enum is provided
  for working with these character classes.

*/


use std::ops::{AddAssign, SubAssign, BitOrAssign, BitAndAssign, BitXorAssign, Add, Sub, BitOr,
               BitAnd, BitXor, Not};
use std::cmp::Ordering;

use super::*;
use crate::debug_log;


// region POSIX Character Classes

// The position of the POSIX class escape in the following string corresponds to the index of the
// POSIX class information table.
pub(crate) const POSIX_CLASS_ESCAPES : &[u8; 28] = b"__sSxX________hHdD__lL__uUwW";

/// We can obtain a Posic class name by index.
pub static POSIX_CLASS_NAMES: [&str; 14] = [
  "ASCII",  // ASCII
  "Space",  // Space : \t-\r, ' '
  "XDigit", // XDigit: 0-9, A-F, a-f
  "Cntrl",  // Cntrl : \x00-0x1F, \0x7F
  "Print",  // Print : ' '-'!'
  "Alnum",  // Alnum : 0-9, A-Z, a-z
  "Alpha",  // Alpha : A-Z, a-z
  "Blank",  // Blank : \t, ' '
  "Digit",  // Digit : 0-9
  "Graph",  // Graph : '!'-'!'
  "Lower",  // Lower : a-z
  "Punct",  // Punct : '!'-'/', ':'-'@', '['-'`', '{'-'!'
  "Upper",  // Upper : A-Z
  "Word",   // Word  : 0-9, A-Z, a-z, _
];

#[allow(non_snake_case)]
pub mod PosixClass {
  #![allow(non_upper_case_globals)]
  use super::*;

  pub static ASCII : Chars = Chars{ b: [ 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0, 0, 0] };
  pub static Space : Chars = Chars{ b: [ 0x0000000100003E00, 0x0000000000000000, 0, 0, 0] };
  pub static XDigit: Chars = Chars{ b: [ 0x03FF000000000000, 0x0000007E0000007E, 0, 0, 0] };
  pub static Cntrl : Chars = Chars{ b: [ 0x00000000FFFFFFFF, 0x8000000000000000, 0, 0, 0] };
  pub static Print : Chars = Chars{ b: [ 0xFFFFFFFF00000000, 0x7FFFFFFFFFFFFFFF, 0, 0, 0] };
  pub static Alnum : Chars = Chars{ b: [ 0x03FF000000000000, 0x07FFFFFE07FFFFFE, 0, 0, 0] };
  pub static Alpha : Chars = Chars{ b: [ 0x0000000000000000, 0x07FFFFFE07FFFFFE, 0, 0, 0] };
  pub static Blank : Chars = Chars{ b: [ 0x0000000100000200, 0x0000000000000000, 0, 0, 0] };
  pub static Digit : Chars = Chars{ b: [ 0x03FF000000000000, 0x0000000000000000, 0, 0, 0] };
  pub static Graph : Chars = Chars{ b: [ 0xFFFFFFFE00000000, 0x7FFFFFFFFFFFFFFF, 0, 0, 0] };
  pub static Lower : Chars = Chars{ b: [ 0x0000000000000000, 0x07FFFFFE00000000, 0, 0, 0] };
  pub static Punct : Chars = Chars{ b: [ 0xFC00FFFE00000000, 0x78000001F8000001, 0, 0, 0] };
  pub static Upper : Chars = Chars{ b: [ 0x0000000000000000, 0x0000000007FFFFFE, 0, 0, 0] };
  pub static Word  : Chars = Chars{ b: [ 0x03FF000000000000, 0x07FFFFFE87FFFFFE, 0, 0, 0] };
}

/// This array allows us to select a Posixx class by index.
pub static POSIX_CLASSES: [&Chars; 14] =
[
  &PosixClass::ASCII,  // ASCII
  &PosixClass::Space,  // Space: \t-\r, ' '
  &PosixClass::XDigit, // XDigit: 0-9, A-F, a-f
  &PosixClass::Cntrl,  // Cntrl: \x00-0x1F, \0x7F
  &PosixClass::Print,  // Print: ' '-'!'
  &PosixClass::Alnum,  // Alnum: 0-9, A-Z, a-z
  &PosixClass::Alpha,  // Alpha: A-Z, a-z
  &PosixClass::Blank,  // Blank: \t, ' '
  &PosixClass::Digit,  // Digit: 0-9
  &PosixClass::Graph,  // Graph: '!'-'!'
  &PosixClass::Lower,  // Lower: a-z
  &PosixClass::Punct,  // Punct: '!'-'/', ':'-'@', '['-'`', '{'-'!'
  &PosixClass::Upper,  // Upper: A-Z
  &PosixClass::Word    // Word: 0-9, A-Z, a-z, _
];


fn escape_to_index(c: Char) -> Option<usize>{
  POSIX_CLASS_ESCAPES.iter().position(| &x | x == Into::<u8>::into(c) )
}


pub fn find_posix_class(c: Char) -> Option<&'static Chars> {
  if let Some(index) = escape_to_index(c){
    Some(POSIX_CLASSES[index/2])
  } else {
    None
  }
}


/**
  Checks to see if `c` is an escape representing one of the POSIX character classes and, if so,
  adds the associated characters to any `Chars` in `maybe_chars` and returns the `CharClass`.
*/
pub fn add_posix_class(c: Char, maybe_chars: &Option<&mut Chars>) -> Option<&'static Chars> {
  let result = escape_to_index(c);

  if let Some(class_index) = result{
    let posix_class = POSIX_CLASSES[class_index/2];

    if let Some(&mut mut chars) = maybe_chars {
      debug_log!("posix({})", POSIX_CLASS_NAMES[class_index/2]);
      chars |= *posix_class;
      // Uppercase means, "anything NOT in the class", so we "flip" the bits, selecting all
      // chars not in the class.
      if c.is_uppercase() {
        // todo: Shouldn't we only flip the characters in the posix class?
        //       See https://github.com/Genivia/RE-flex/issues/82
        chars.flip();
      }
    }

    Some(posix_class)
  }
  else{
    None
  }
}


// endregion


/// Set of chars and meta chars
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Chars {
  pub b: [u64; 5] //< 256 bits for chars + bits for meta
}


impl Chars {

  pub fn new() -> Chars{
    Chars{
      b: [0; 5]
    }
  }


  pub fn from_u64(c: &[u64; 5]) -> Chars {
    Chars{
      b: *c
    }
  }


  pub fn from_vec(v: &Vec<Char>) -> Chars {
    let mut result = Chars::new();
    for c in v {
      result.insert(*c);
    }

    result
  }


  pub fn clear(&mut self) {
    self.b = [0; 5];
  }


  pub fn is_empty(&self)  -> bool {
    self.b[0] == 0 &&
    self.b[1] == 0 &&
    self.b[2] == 0 &&
    self.b[3] == 0 &&
    self.b[4] == 0
  }


  pub fn intersects(&self, c: &Chars) -> bool {
    !(
      (self.b[0] & c.b[0]) == 0 &&
      (self.b[1] & c.b[1]) == 0 &&
      (self.b[2] & c.b[2]) == 0 &&
      (self.b[3] & c.b[3]) == 0 &&
      (self.b[4] & c.b[4]) == 0
    )
  }


  pub fn is_subset(&self, c: &Chars) -> bool {
    return (*c - *self).is_empty();
  }


  pub fn contains(&self, c: Char) -> bool {
    return (self.b[(c.0 >> 6) as usize] & (1 << (c.0 & 0x3F))) != 0
  }


  pub fn insert(&mut self, c: Char) -> &Chars {
    self.b[(c.0 >> 6) as usize] |= 1 << (c.0 & 0x3F);
    return self;
  }


  pub fn insert_pair(&mut self, lo: Char, hi: Char) -> &Chars {
    for c in lo.0..=hi.0 {
      self.insert(Char(c));
    };
    return self;
  }


  // todo: This does not appear to be used and can easily be confused with the more common flip256.
  /*
  /// Computes `!self` in-place.
  pub fn flip_all(&mut self) -> &Chars {
    self.b[0] = !self.b[0];
    self.b[1] = !self.b[1];
    self.b[2] = !self.b[2];
    self.b[3] = !self.b[3];
    self.b[4] = !self.b[4];
    return self;
  }
  */

  /// Same as `flip_all()` but omits the meta characters `self.b[4]`.
  pub fn flip(&mut self) -> &Chars {
    self.b[0] = !self.b[0];
    self.b[1] = !self.b[1];
    self.b[2] = !self.b[2];
    self.b[3] = !self.b[3];
    return self;
  }


  pub fn swap(&mut self, c: &mut Chars) -> &mut Chars {
    // let t: &mut Chars = c;
    // c = self;
    // self = t;
    std::mem::swap(self, c);
    return self;
  }


  pub fn lo(&self) -> Char {
    for i in 0..5 {
      if self.b[i] != 0 {
        for j in 0..64 {
          if (self.b[i] & (1 << j)) != 0 {
            // The index is the upper 3 bits and the bit number is the lowest 6 bits.
            return Char::from((i << 6)  + j);
          }
        }
      }
    }
    return Char(0);
  }


  pub fn hi(&self) -> Char {
    for i in 0..5 {
      // Find the largest index for which...
      if self.b[4 - i] != 0 {
        for j in 0..64 {
          // todo: use `leading_zeroes()`
          // ... we find the highest set bit.
          if (self.b[4 - i] & (1 << (63 - j))) != 0 {
            // Reconstruct the number.
            // The index is the upper 3 bits and the bit number is the lowest 6 bits.
            return Char::from(((4 - i) << 6) + (63 - j));
          }
        }
      }
    }
    return Char(0);
  }


  /// Adds uppercase versions of all lowercase `Chars` and vice versa.
  pub fn make_case_insensitive(&mut self) {
    let lower = *self & PosixClass::Lower.into();
    let upper = *self & PosixClass::Upper.into();

    for c in lower.into_iter().chain(upper.into_iter()) {
      self.insert(c.toggle_case());
    }

  }

}



// region Arithmetic for Chars

impl AddAssign for Chars{
  fn add_assign(&mut self, c: Chars) {
    return self.bitor_assign(c);
  }
}


impl SubAssign for Chars{
  fn sub_assign(&mut self, c: Chars) {
    self.b[0] &= !c.b[0];
    self.b[1] &= !c.b[1];
    self.b[2] &= !c.b[2];
    self.b[3] &= !c.b[3];
    self.b[4] &= !c.b[4];
  }
}


impl BitOrAssign for Chars{
  fn bitor_assign(&mut self, c: Chars) {
    self.b[0] |= c.b[0];
    self.b[1] |= c.b[1];
    self.b[2] |= c.b[2];
    self.b[3] |= c.b[3];
    self.b[4] |= c.b[4];
  }
}


impl BitAndAssign for Chars{
  fn bitand_assign(&mut self, c: Chars) {
    self.b[0] &= c.b[0];
    self.b[1] &= c.b[1];
    self.b[2] &= c.b[2];
    self.b[3] &= c.b[3];
    self.b[4] &= c.b[4];
  }
}


impl BitXorAssign for Chars{
  fn bitxor_assign(&mut self, c: Chars) {
    self.b[0] ^= c.b[0];
    self.b[1] ^= c.b[1];
    self.b[2] ^= c.b[2];
    self.b[3] ^= c.b[3];
    self.b[4] ^= c.b[4];
  }
}


impl Add for Chars{
  type Output = Chars;

  fn add(self, c: Chars) -> Chars {
    let mut copy = self;
    copy += c;
    return copy;
  }
}


impl Sub for Chars{
  type Output = Chars;

  fn sub(self, c: Chars) -> Chars {
    let mut copy = self;
    copy -= c;
    return copy;
  }

}


impl BitOr for Chars{
  type Output = Chars;

  fn bitor(self, c: Chars) -> Chars {
    let mut copy = self;
    copy |= c;
    return copy;
  }

}


impl BitAnd for Chars{
  type Output = Chars;

  fn bitand(self, c: Chars) -> Chars {
    type Output = Chars;

    let mut copy = self;
    copy &= c;
    return copy;
  }

}


impl BitXor for Chars{
  type Output = Chars;

  fn bitxor(self, c: Chars) -> Chars {
    let mut copy = self;
    copy ^= c;
    return copy;
  }

}


impl Not for Chars{
  type Output = Chars;

  // todo: Replace with `flip()` (originally `flip256()`)?
  fn not(self) -> Chars {
    let mut copy: Chars = self;
    copy.flip();
    return copy;
  }

}


//
// impl Eq for Chars{
//   fn eq(&mut self, c: &Chars) -> bool {
//     return (self.b[0] == c.b[0]) &&
//            (self.b[1] == c.b[1]) &&
//            (self.b[2] == c.b[2]) &&
//            (self.b[3] == c.b[3]) &&
//            (self.b[4] == c.b[4]);
//   }
//
// }

impl PartialOrd for Chars{
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(Chars::cmp(self, other))
  }
}


impl Ord for Chars{
  fn cmp(&self, c: &Chars) -> Ordering {
    let equal:bool =
        self.b[0] == c.b[0] &&
        self.b[1] == c.b[1] &&
        self.b[2] == c.b[2] &&
        self.b[3] == c.b[3] &&
        self.b[4] == c.b[4];

    if equal {
      return Ordering::Equal;
    }

    match self.b[0] < c.b[0] ||
            (self.b[0] == c.b[0] &&
              (self.b[1] < c.b[1] ||
                (self.b[1] == c.b[1] &&
                  (self.b[2] < c.b[2] ||
                    (self.b[2] == c.b[2] &&
                      (self.b[3] < c.b[3] ||
                        (self.b[3] == c.b[3] && self.b[4] < c.b[4])
                      )
                    )
                  )
                )
              )
            ){

      true  => Ordering::Less,

      false => Ordering::Greater

    }
  }
}

// endregion


// region `impl` traits for Chars


impl From<CharsIterator> for Chars {
  fn from(iter: CharsIterator) -> Self {
    let mut result = Chars::new();
    for c in iter {
      result.insert(c);
    }
    result
  }
}

impl From<Vec<u8>> for Chars {
  fn from(v: Vec<u8>) -> Self {
    let mut result = Chars::new();
    for c in v {
      result.insert(Char::from(c));
    }
    result
  }
}

// endregion


// region `CharsIterator`


impl IntoIterator for Chars{
  type Item     = Char;
  type IntoIter = CharsIterator;

  /// The resulting iterator yeilds values, not references, and does not consume the Chars.
  fn into_iter(self) -> Self::IntoIter {
    CharsIterator::new(self)
  }
}


pub struct CharsIterator{
  chars: Chars,
  next_char: Char
}

impl CharsIterator {
  pub fn new(c: Chars) -> CharsIterator {
    CharsIterator{
      chars: c,
      next_char: Char(0)
    }
  }
}

impl Iterator for CharsIterator{
  type Item = Char;

  fn next(&mut self) -> Option<Self::Item> {
    if self.next_char.0 >= Meta::MAX.0  //Into::<u16>.into(Meta::MAX)
    {
      return None;
    }

    for c in self.next_char.0..Meta::MAX.0{
      if self.chars.contains(Char(c)){
        self.next_char = Char(c + 1);
        return Some(Char(c));
      }
    }
    self.next_char = Meta::MAX;
    None
  }
}

// endregion
