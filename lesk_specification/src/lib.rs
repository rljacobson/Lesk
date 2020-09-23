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
// mod code;
mod section_items;
mod mergable;

use std::collections::{HashMap, HashSet};

pub use options::Options;
pub use specification::Specification;
pub use crate::parser::{Span, source::FileId};

use crate::parser::source;

type SourceFiles = source::SourceFiles<String>;
type SourceFile = source::SourceFile<String>;

// todo: AddList?
type Code = Vec<Span>;                //< Collection of ordered lines of code
type CodeMap = HashMap<Start, Code>;    //< Map of start conditions to lines of code
type Dictionary<'s> = HashMap<String, &'s str>; //< Dictionary (const char*)
// todo: AddList?
type Rules      = Vec<Rule>;            //< Collection of ordered rules
type RulesMap   = HashMap<Start, Rule>; //< Map of start conditions to rules
type Start          = usize;                    //< Start condition state type
type Starts         = HashSet<Start>;           //< Set of start conditions
type StrMap<'s>     = HashMap<&'s str, &'s str>;//< Dictionary (std::string)
// todo: AddList?
type StrVec<'s>     = Vec<&'s str>;             //< Collection of ordered strings


// todo: Write the RegexEngine struct
#[derive(Default)]
struct RegexEngine;


/// A regex pattern and action pair that forms a rule
struct Rule {
  pattern : Span,    //< the pattern
  regex   : String,  //< the pattern-converted regex for the selected regex engine
  code    : Span     //< the action code corresponding to the pattern
}

