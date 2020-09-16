/*!
Representations of the various items that can appear in Section 1, the definitions section.
*/

use super::{SourceFile, Code};
use crate::options::OptionSet;
use std::fmt::Display;
use nom::lib::std::fmt::Formatter;

pub type SectionOneItemSet  = Vec<SectionOneItem>;

#[derive(Clone, Debug)]
pub enum SectionOneItem {
  User(Code),
  Top(Code),
  Class(Code),
  Init(Code),
  Unknown(Code),
  Include {
    file: SourceFile,
    contents: SectionOneItemSet,
  },
  State {
    is_exclusive: bool,
    code: Code,
  },
  Options(OptionSet),
}

impl Display for SectionOneItem{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {

    let text =
    match self {
      SectionOneItem::User(code) => {
        format!("User({:?})\n", code)
      },
      SectionOneItem::Top(code) => {
        format!("Top({:?})\n", code)
      },
      SectionOneItem::Class(code) => {
        format!("Class({:?})\n", code)
      },
      SectionOneItem::Init(code) => {
        format!("Init({:?})\n", code)
      },
      SectionOneItem::Unknown(code) => {
        format!("Unknown({:?})\n", code)
      },
      SectionOneItem::Include {
        file,
        contents,
      } => {
        format!("Include{{ file={:?} }}\n", file.name())
      },
      SectionOneItem::State {
        is_exclusive,
        code
      } => {
        format!("State{{is_exlusive: {:?}, code={:?} }}", is_exclusive, code)
      },
      SectionOneItem::Options(option_set) => {
        format!("Options:\n{:?}\n", option_set)
      },
    };

    write!(f, "{}", text)
  }
}


// Merges `lhs` and `rhs` into a single `SectionOneItemSet`, consuming both original sets.
// pub fn merge_item_sets(lhs: SectionOneItemSet, rhs: SectionOneItemSet){
//   rhs.drain().map(| (name, item) | {
//     lhs[name].
//   })
// }
