
#![allow(dead_code)]


mod limits;
pub mod modifier;
pub mod parser;
pub mod options;
pub mod error;
pub mod debug;
pub mod chars;
pub mod opcode;
pub mod character;
pub mod position;
pub mod group;

use std::collections::{HashMap, HashSet};

use ranges::Ranges;

use position::{Position, PositionSet};
use chars::Chars;
use character::*;
use defaultmap::DefaultHashMap;


// We alias types to enforce size restrictions on their values.
pub type Lazy8    = u8;  //< Lazy values
type GroupIndex32 = u32; //< Capture/match group numbers
type Hash16       = u16; //< Hash value type having max value `Const::HASH`
type Index32      = u32; //< An index into the regex string
type Iteration16  = u16; //< Iteration values
type Lookahead16  = u16; //???
type PredictBits8 = u8;  //< Predict match bits

// Containers of the above.
// todo: Is AppendList better than HashMap for these?
type MoveVec     = Vec<Move>;
type LazySet     = HashSet<Lazy8>; //< Positions within the regex that are lazily matched?
type Move        = (Chars, PositionSet); //< Analogous to an `Edge`, but without a `State`
type IndexRanges = Ranges<Index32>;
type MapToRanges = HashMap<Index32, IndexRanges>;
type FollowMap   = DefaultHashMap<Position, PositionSet>;
