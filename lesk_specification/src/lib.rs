#![feature(iterator_fold_self)]
#![feature(drain_filter)]
/*!

Utilities to parse a lexer specification file and create an abstract representation of the files
contents.

*/
#[macro_use]
extern crate phf;

mod options;
mod specification;
mod error;
mod parser;
mod section_items;
mod mergable;
mod mode;

use std::collections::{HashMap, HashSet};

pub use options::Options;
pub use specification::Specification;
pub use crate::parser::source::Span;
use crate::parser::source::{SourceFiles, SourceID};


type Code<'s>       = Vec<Span<'s>>;             //< Collection of ordered lines of code
type CodeMap<'s>    = HashMap<Start, Code<'s>>;  //< Map of start conditions to lines of code
type Dictionary<'s> = HashMap<String, &'s str>;  //< Dictionary (const char*)
type Rules<'a>      = Vec<Rule<'a>>;             //< Collection of ordered rules
type RulesMap<'a>   = HashMap<Start, Rule<'a>>;  //< Map of start conditions to rules
type Start          = usize;                     //< Start condition state type
type Starts         = HashSet<Start>;            //< Set of start conditions
type StrMap<'s>     = HashMap<&'s str, &'s str>; //< Dictionary (std::string)
type StrVec<'s>     = Vec<&'s str>;              //< Collection of ordered strings



// todo: Write the RegexEngine struct
#[derive(Default)]
struct RegexEngine;


/// A regex pattern and action pair that forms a rule
struct Rule<'a> {
  pattern : Span<'a>, //< the pattern
  // regex   : String,   //< the pattern-converted regex for the selected regex engine
  code    : Span<'a>  //< the action code corresponding to the pattern
}

