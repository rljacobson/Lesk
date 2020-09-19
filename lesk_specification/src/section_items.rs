/*!
Representations of the various items that can appear in Section 1, the definitions section.
*/

use std::fmt::Display;

use nom::lib::std::fmt::Formatter;

use crate::mergable::{Mergable, Merged};
use crate::options::OptionSet;
use crate::parser::Span;

use super::{Code, SourceFile};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum ItemType {
  User,
  Top,
  Class,
  Init,
  Unknown,
  Include,
  State,
  Options,
}

impl ItemType {
  pub fn new(&self, item: span) -> SectionItem {
    match self {
      ItemType::Top => SectionItem::top_code(item),
      ItemType::Class => SectionItem::class_code(item),
      ItemType::Init => SectionItem::init_code(item),
      ItemType::User => SectionItem::user_code(item),
      ItemType::Unknown => SectionItem::unknown_code(item),
      _ => {
        unreachable!("Cannot use ItemType::new to create an Options, Include, or State.");
      }
      // ItemType::Options => SectionItem::options_code(item),
      // ItemType::Include => SectionItem::Include(item),
      // ItemType::State => SectionItem::State(item)
    }
  }

  pub fn open_delimiter(&self) -> &'static str {
    match self {
      ItemType::Top => "%top{",
      ItemType::Class => "%class{",
      ItemType::Init => "%init{",
      ItemType::User => "%{",
      ItemType::Unknown => "{",
      ItemType::Include => "%include",
      ItemType::Options => "%options",

      ItemType::State => {
        // This method is never called on `SectionItem::State`
        panic! {"SectionItem::State has multiple opening delimiters."};
      }
    }
  }

  pub fn is_code(&self) -> bool {
    match self {
      | ItemType::Top
      | ItemType::Class
      | ItemType::Init
      | ItemType::User
      | ItemType::Unknown => true,

      | ItemType::Include
      | ItemType::Options
      | ItemType::State => false
    }
  }

  // For symmetry with `open_delimiter`
  pub fn close_delimiter(&self) -> &'static str {
    match self {
      | ItemType::Top
      | ItemType::Class
      | ItemType::Init
      | ItemType::User
      | ItemType::Unknown => "}",

      | ItemType::Include
      | ItemType::Options
      | ItemType::State => ""
    }
  }
}

pub type SectionItemSet = Vec<SectionItem>;

#[derive(Clone, Debug)]
pub enum SectionItem {
  User(Code),
  Top(Code),
  Class(Code),
  Init(Code),
  Unknown(Code),
  Include {
    file: SourceFile,
    contents: SectionItemSet,
  },
  State {
    is_exclusive: bool,
    code: Code,
  },
  Options(OptionSet),
}

impl Display for SectionItem {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    let text =
        match self {
          SectionItem::User(code) => {
            format!("User({:?})\n", code)
          }
          SectionItem::Top(code) => {
            format!("Top({:?})\n", code)
          }
          SectionItem::Class(code) => {
            format!("Class({:?})\n", code)
          }
          SectionItem::Init(code) => {
            format!("Init({:?})\n", code)
          }
          SectionItem::Unknown(code) => {
            format!("Unknown({:?})\n", code)
          }
          SectionItem::Include {
            file,
            contents,
          } => {
            format!("Include{{ file={:?} }}\n", file.name())
          }
          SectionItem::State {
            is_exclusive,
            code
          } => {
            format!("State{{is_exlusive: {:?}, code={:?} }}", is_exclusive, code)
          }
          SectionItem::Options(option_set) => {
            format!("Options:\n{:?}\n", option_set)
          }
        };

    write!(f, "{}", text)
  }
}

impl SectionItem {
  pub fn user_code<S>(span: S) -> Self
    where S: Into<Span>
  {
    SectionItem::User(vec![span])
  }

  pub fn top_code<S>(span: S) -> Self
    where S: Into<Span>
  {
    SectionItem::Top(vec![span])
  }

  pub fn class_code<S>(span: S) -> Self
    where S: Into<Span>
  {
    SectionItem::Class(vec![span])
  }

  pub fn init_code<S>(span: S) -> Self
    where S: Into<Span>
  {
    SectionItem::Init(vec![span])
  }

  pub fn unknown_code<S>(span: S) -> Self
    where S: Into<Span>
  {
    SectionItem::Unknown(vec![span])
  }

  // Supplies the `ItemType` variant associated to this `SectionItem`.
  pub fn item_type(&self) -> ItemType {
    match self {
      SectionItem::Top(_) => ItemType::Top,
      SectionItem::Class(_) => ItemType::Class,
      SectionItem::Init(_) => ItemType::Init,
      SectionItem::User(_) => ItemType::User,
      SectionItem::Unknown(_) => ItemType::Unknown,
      SectionItem::Include { .. } => ItemType::Include,
      SectionItem::Options(_) => ItemType::Options,
      SectionItem::State { .. } => ItemType::State,
    }
  }

  pub fn is_code(&self) -> bool {
    self.item_type().is_code()
  }

  pub fn open_delimiter(&self) -> &'static str {
    self.item_type().open_delimiter()
  }

  // For symmetry with `open_delimiter`
  pub fn close_delimiter(&self) -> &'static str {
    self.item_type().close_delimiter()
  }

  // Pushes a `span` onto the `code` vector when `self` is a code-wrapping `SectionOneItem`.
  // Panics otherwise.
  pub fn push_code<S>(&mut self, span: S)
    where S: Into<Span>
  {
    match self {
      | SectionItem::User(code)
      | SectionItem::Top(code)
      | SectionItem::Class(code)
      | SectionItem::Init(code)
      | SectionItem::State { code, .. }
      | SectionItem::Unknown(code) => {
        code.push(span.into());
      }

      | SectionItem::Include { .. }
      | SectionItem::Options(_) => {
        panic!("Tried to push {} onto code.", self);
      }
    }
  }

  pub fn into_code(self) -> Code {
    match self {
      | SectionItem::User(code)
      | SectionItem::Top(code)
      | SectionItem::Class(code)
      | SectionItem::Init(code)
      | SectionItem::State { code, .. }
      | SectionItem::Unknown(code) => code,

      | SectionItem::Include { .. }
      | SectionItem::Options(_) => {
        panic!("Tried to turn {} into code.", self);
      }
    }
  }


  pub fn get_code(&mut self) -> Option<&mut Code> {
    match self {
      | SectionItem::User(code)
      | SectionItem::Top(code)
      | SectionItem::Class(code)
      | SectionItem::Init(code)
      | SectionItem::State { code, .. }
      | SectionItem::Unknown(code) => Some(code),

      | SectionItem::Include { .. }
      | SectionItem::Options(_) => {
        None
      }
    }
  }
}


impl Mergable for SectionItem {


  fn mergable(&self, other: &SectionItem) -> bool {
    (self.item_type() == other.item_type()) && self.is_code()
  }


  /**
  Attempts tp merge `self` with `other`. This method is asymmetric: it assumes `self` was parsed
  before `other`.
  */
  fn merged(mut self, mut other: SectionItem) -> Merged<SectionItem, SectionItem> {
    if self.item_type() == other.item_type() {
      match &mut self {
        | SectionItem::User(self_code)
        | SectionItem::Top(self_code)
        | SectionItem::Class(self_code)
        | SectionItem::Init(self_code)
        | SectionItem::Unknown(self_code)
        => {
          // Unwrap always succeeds because of very first `if` above.
          let other_code = other.get_code().unwrap();

          if self_code.is_empty() {
            return Merged::Yes(other);
          } else if other_code.is_empty() {
            return Merged::Yes(self);
          }

          // Unwraps always succeed because of preceding `is` block.
          let mut self_last_span = self_code.last_mut().unwrap();
          let mut other_first_span = other_code.first_mut().unwrap();
          match self_last_span.merged(other_first_span) {
            Merged::No(_, _) => {
              // Can still "merge" the vectors
              self_code.append(other_code);
              Merged::Yes(self)
            }

            Merged::Yes(merged) => {
              self_code.pop();
              self_code.push(merged);
              self_code.extend(other_code.drain(1..));
              Merged::Yes(self)
            }
          }
        }

        | SectionItem::State
        | SectionItem::Include
        | SectionItem::Options(_) => Merged::No(self, other)

      } // end match self

    }
    else {
      Merged::No(self, other)
    }
  }
}
