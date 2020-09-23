use codespan::Files;

pub mod parser;
pub mod source;
mod whitespace;
mod span;

pub use span::*;
pub use super::options::{OptionSet, OptionField};


pub type InputType<'a> = LSpan<'a>;
