#![allow(dead_code)]

/*!


| Mneumonic | Opcode         |  Binary      |  Decimal | Payload        | Comments |
|:--------- |:--------------:|:------------:|:--------:|:---------------|:--------|
| HALT      | `0x_00_FF_FF_FF` |              |          | Part of opcode |   |
| HEAD      | `0x_FB_00_00_00` | `0b1111_1011` | `251`    | Index |   |
| TAIL      | `0x_FC_00_00_00` | `0b1111_1100` | `252`    | Index |   |
| REDO      | `0x_FD_00_00_00` | `0b1111_1101` | `253`    | No payload | Same as GOTO8  |
| TAKE      | `0x_FE_00_00_00` | `0b1111_1110` | `254`    | Index |   |
| LONG      | `0x_FF_00_00_00` | `0b1111_1111` | `255`    | Index | Same as GOTO  |
| GOTO      | `0x_FF_00_00_00` | `0b1111_1111` | `255`    |         | Same as LONG  |
| GOTO8     | `0x_FD_00_00_00` | `0b1111_1101` | `253`    |         | Same as REDO  |

# Details

We will use the following notation. In hex, we have a 32 bit number with bytes labeled `Bn` for the
nth byte and hex digit places (half-bytes or nibbles) numbered 1 through 8:

```
0x B4 B3 B2 B1
   -- -- -- --
   87 65 43 21
```



## `GOTO`'s vs other instructions.

The opcodes are chosen to be large numbers, close to 0xFF, and their payloads (typically indices)
are restricted to 3-byte numbers with the 3rd byte less than the opcode. Thus, `B3 >= B4` if and
only if the instruction is a `GOTO`.

For meta characters, `B3==0`, while `B4 <= 0x0D == 13`, much smaller than any other opcode.

This is not the simplest scheme and puts awkward restrictions on the maximum index that can be
stored in instructions, so I wonder why it was chosen.

## `GOTO`'s encoding of Chars

The `GOTO` instruction embeds characters: either a single meta character, or a `lo`/`hi` pair.

### ASCII character ranges from `lo` to `hi`

In hex, we have the following 32 bit number with bytes labeled `Bn` for the nth byte and hex digit
places (half-bytes or nibbles) numbered:

```
0x B4 B3 B2 B1         0x 00 B3       0x 00 B4
   -- -- -- -- ==> hi:    -- --  lo:     -- --
   87 65 43 21            43 21          43 21
```

We have `B4 = lo` and `B3 = hi`. Therefore, it is always the case that `B4<=B3`. Note that this
condition never holds for meta characters (see below).

### Meta characters

As before, we have:

```
0x B4 B3 B2 B1     0x 01 B4
   -- -- -- -- ==>    -- --
   87 65 43 21        43 21
```

For `is_meta()` to be true, we must have `B4 > 0` and `B3 == 0`.\* (Note that this condition never
holds for ASCII characters.)

Then the meta character is given by `Char(0x01B4)` where `B4` is standing in for the actual hex
digits in the 7th and 8th places which make up `B4`.

\* In fact, more is true: We must have `0 <= B4 <= 0x0D`, the second inequality distinguishing meta
characters from other instructions in which `B3` happens to be zero.

*/

use std::fmt::{Display, Formatter};

use super::*;

pub mod bitmasks {
  use super::Index32;

  // opcodes
  pub const HALT  : u32 = 0x00FFFFFFu32;  //< The halt opcode
  pub const HEAD  : u32 = 0xFB000000u32;  // 0b1111_1011 <-- highest byte = 251
  pub const TAIL  : u32 = 0xFC000000u32;  // 0b1111_1100 <-- highest byte = 252
  pub const REDO  : u32 = 0xFD000000u32;  // 0b1111_1101 <-- highest byte = 253
  pub const TAKE  : u32 = 0xFE000000u32;  // 0b1111_1110 <-- highest byte = 254
  pub const LONG  : u32 = 0xFF000000u32;  // 0b1111_1111 <-- highest byte = 255 same as goto
  pub const GOTO  : u32 = 0xFF000000u32;  // 0b1111_1111 <-- highest byte = 255 same as long
  pub const GOTO8 : u32 = 0xFD000000u32;  // 0b1111_1101 <-- highest byte = 253 same as redo


  pub const LONG_MARKER: Index32  = 0xFFFE; //< LONG marker for 64 bit opcodes, must be HALT - 1
  pub const HALT_MARKER: Index32  = 0xFFFF; //< HALT marker for GOTO opcodes, must be 16 bit max

  // Byte masks
  pub const BYTE1   : u32 = 0x000000FFu32;  // Mask first 8 bits
  pub const BYTE1_2 : u32 = 0x0000FFFFu32;  // Mask first 16 bits (first and second bytes)
  pub const BYTE1_3 : u32 = 0x00FFFFFFu32;  // Mask first 24 bits (first, second, third bytes)
  pub const BYTE3   : u32 = 0x00FF0000u32;  // Mask third byte (bits 16-23)
  pub const BYTE4   : u32 = 0xFF000000u32;  // Mask high byte (bits 24-31)

  // Mask off opcode
  pub const OPCODE     : u32 = BYTE4;
  // Payload masks
  pub const INDEX      : u32 = BYTE1_2;  // Lowest two bytes
  pub const LOOKAHEAD  : u32 = BYTE1_2;  // Lowest two bytes
  pub const LONG_INDEX : u32 = BYTE1_3;  // Lowest three bytes

  // Bitshift amounts
  pub const BYTE4_SHIFT: u32 = 24;  // Shift to get highest byte (most significant)
  pub const BYTE3_SHIFT: u32 = 16;  // Shift to get 3rd and 4th bytes, bits 16-31.

}

use bitmasks::*;
use character::{Meta, Char};

/// 32-bit opcode word
pub struct Opcode(u32);

// todo: Make constants/enum for these bit masks.
impl Opcode {

  /// Returns the redo opcode, a constant
  pub fn redo() -> Opcode {
    return Opcode(REDO);
  }

  /// Returns the halt opcode, a constant
  pub fn halt() -> Opcode {
    return Opcode(HALT);
  }

  pub fn is_long(&self) -> bool {
    return (self.0 & OPCODE) == LONG_MARKER;
  }

  pub fn is_take(&self) -> bool {
    // todo: Matches TAKE, LONG, and GOTO?? Possibly typo.
    //return (self.0 & 0xFE000000u32) == 0xFE000000u32;
    // Tentative fix for presumed typo above
    return (self.0 & OPCODE) == TAKE;
  }

  pub fn is_redo(&self) -> bool {
    return self.0 == REDO;
  }

  pub fn is_tail(&self) -> bool {
    return (self.0 & OPCODE) == TAIL;
  }

  pub fn is_head(&self) -> bool {
    return (self.0 & OPCODE) == HEAD;
  }

  pub fn is_halt(&self) -> bool {
    return self.0 == HALT;
  }

  /**
    Note that `is_goto()` gives false for meta characters, which have their own `is_meta()`.

    ```
      87 65 43 21  (top numbers/answers are place labels, not digits.)
             << 8
      -----------
      65 43 21 00
               >=
      87 65 43 21
    & FF 00 00 00
    -------------
      87 00 00 00
    ```
    True if third byte is >= fourth byte.

    RJ: Isn't this also true of, e.g., LONG?
     A: No, because we artificially restrict the payload's 3rd byte to be less than the opcode.
  */
  pub fn is_goto(&self) -> bool {
    return (self.0 & BYTE3) >> BYTE3_SHIFT >= (self.0 & BYTE4) >> BYTE4_SHIFT;
  }

  pub fn is_meta(&self) -> bool {
    return ((self.0 & BYTE3) == 0) && ((self.0 >> BYTE4) > 0);
  }

  /*
    Determines if the character `c` is in the range `lo` to `hi`. Note that this implies
    automatically that the instruction is a goto.
   */
  pub fn is_goto_u8(&self, c: u8) -> bool {
    /*
      c >= fourth byte
      c <= third byte
    */

    c>= ((self.0 >> BYTE4_SHIFT) as u8) &&
    c <= (((self.0 >> BYTE3_SHIFT) & BYTE1) as u8)
  }

  /// Decodes the meta character encoded in the GOTO instruction. Does no sanity checking to see
  /// if the instruction is a GOTO instruction, the character is a meta character, etc.
  pub fn meta(&self) -> Char {
    /*
      Normally, meta characters are 0x0101 through 0x010D, but in opcodes they are stored as 0x01
      through 0x0D in the fourth byte. So recover 4th byte and add 0x0100.
    */
    return Char(Meta::MIN.0 + ((self.0 >> BYTE4_SHIFT) as u16));
  }

  /// Decodes the `lo` character encoded in the instruction.
  pub fn lo(&self) -> Char {
    match self.is_meta() {
      true  => {
        self.meta()
      }

      false => {
        Char((self.0 >> BYTE4_SHIFT) as u16)
      }
    }
  }

  /// Decodes the `hi` character encoded in the GOTO instruction.
  pub fn hi(&self) -> Char {
    match self.is_meta() {
      true  => {
        self.meta()
      }

      false => {
        Char(((self.0 >> BYTE3_SHIFT) & BYTE1) as u16)
      }
    }
  }

  pub fn idx(&self) -> Index32 {
    return self.0 & INDEX;
  }

  pub fn long_idx(&self) -> Index32 {
    return self.0 & LONG_INDEX;
  }

  pub fn lookahead(&self) -> Lookahead16 {
    return (self.0 & LOOKAHEAD) as Lookahead16;
  }


} // end impl Opcode

impl Display for Opcode{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}


// region Opcode Construction Functions

pub fn opcode_long(index: Index32) -> Opcode {
  return Opcode(LONG_MARKER | (index & LONG_INDEX));
  // index <= Const::GMAX (max: 0xFEFFFFu32)
}

pub fn opcode_take(index: Index32) -> Opcode {
  return Opcode(TAKE | (index & LONG_INDEX));
  // index <= Const::AMAX (max: 0xFDFFFFu32)
}


pub fn opcode_tail(index: Index32) -> Opcode {
  return Opcode(TAIL | (index & LONG_INDEX));
  // index <= Const::LMAX (max: 0xFAFFFFu32)
}

pub fn opcode_head(index: Index32) -> Opcode {
  return Opcode(HEAD | (index & LONG_INDEX));
  // index <= Const::LMAX (max: 0xFAFFFFu32)
}

pub fn opcode_goto(lo: Char, hi: Char, index: Index32) -> Opcode {
  match lo.is_meta() {
    true => {
      Opcode((u32::from(lo) << BYTE4_SHIFT) | index)
    }

    false => {
      Opcode((u32::from(lo) << BYTE4_SHIFT) | (u32::from(hi) << BYTE3_SHIFT) | index)
    }
  }
}

// endregion
