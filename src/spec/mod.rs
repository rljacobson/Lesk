/*!

Utilities to parse a lexer specification file and create an abstract representation of the files
contents.

*/

mod options;
mod specification;

pub use options::Options;
pub use specification::Specification;

use std::collections::{HashMap, HashSet};

use codespan::Span as Code;

// todo: AddList?
type Codes          = Vec<Code>;                //< Collection of ordered lines of code
type CodesMap       = HashMap<Start, Codes>;    //< Map of start conditions to lines of code
type Dictionary<'s> = HashMap<String, &'s str>; //< Dictionary (const char*)
// todo: AddList?
type Rules<'r>      = Vec<Rule<'r>>;            //< Collection of ordered rules
type RulesMap<'r>   = HashMap<Start, Rule<'r>>; //< Map of start conditions to rules
type Start          = usize;                    //< Start condition state type
type Starts         = HashSet<Start>;           //< Set of start conditions
type StrMap<'s>     = HashMap<&'s str, &'s str>;//< Dictionary (std::string)
// todo: AddList?
type StrVec<'s>     = Vec<&'s str>;             //< Collection of ordered strings

// todo: Write the Library struct
#[derive(Default)]
struct Library;


/// A regex pattern and action pair that forms a rule
struct Rule<'r> {
  pattern : &'r str, //< the pattern
  regex   : &'r str, //< the pattern-converted regex for the selected regex engine
  code    : Code     //< the action code corresponding to the pattern
}

