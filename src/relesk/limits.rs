/**
  Constants specifying limits.

*/


use super::{GroupIndex32, Index32, Iteration16};

/// Max number of iterations of a repeatable, e.g. `a{3,5}`.
pub(crate) static MAX_ITER: Iteration16 = u16::MAX;

/// The first 32 bits of a `Position` hold the `Position`'s index into the regex. In some
/// contexts there are other restrictions on the maximum index. See below.
pub const MAX_INDEX : Index32 = u32::MAX;

// Opcode/Instruction Determined Limits
// These maxima exist in order to maintain the invariant within an instruction that byte3 >= byte4
// if and only if the instruction is a `GOTO`. See description of instruction encoding in the
// `opcode` module-level documentation.
pub const IMAX_IDX          : Index32  = 0xFFFFFFFF; //< max index, also serves as a marker
pub const GOTO_MAX_IDX      : Index32  = 0xFEFFFF;   //< max goto index
pub const ACCEPT_MAX        : GroupIndex32 = 0xFDFFFF;   //< max accept
pub const LOOKAHEAD_MAX_IDX : Index32  = 0xFAFFFF;   //< max lookahead index


// Hash Array Limits
// Formerly HASH
pub const HASH_MAX_IDX      : usize    = 0x1000;     //< size of the predict match array (4096)


