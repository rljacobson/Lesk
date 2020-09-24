/*!
Representations of the various items that can appear in Section 1, the definitions section.
*/

// todo: Cut dead code.

use std::fmt::Display;

use nom::lib::std::fmt::Formatter;

use crate::mergable::{Mergable, Merged, merge_or_append_items, merge_or_push_item};
use crate::options::OptionField;
use crate::parser::{Span, ToSpan};

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
  Definition,
  Option,
}

impl Display for ItemType{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    let name =
        match self {
          ItemType::Top => "ItemType::Top",
          ItemType::Class => "ItemType::Class",
          ItemType::Init => "ItemType::Init",
          ItemType::User => "ItemType::User",
          ItemType::Unknown => "ItemType::Unknown",
          ItemType::Include => "ItemType::Include",
          ItemType::Option => "ItemType::Option",
          ItemType::State => "ItemType::State",
          ItemType::Definition => {"ItemType::Definition"}
        };

    write!(f, "{}", name)
  }
}

impl ItemType {
  pub fn new<S>(&self, item: S) -> SectionItem
      where S: ToSpan
  {
    match self {
      ItemType::Top => SectionItem::top_code(item.to_span()),
      ItemType::Class => SectionItem::class_code(item.to_span()),
      ItemType::Init => SectionItem::init_code(item.to_span()),
      ItemType::User => SectionItem::user_code(item.to_span()),
      ItemType::Unknown => SectionItem::unknown_code(item.to_span()),

      _ => {
        unreachable!("Cannot use ItemType::new to create an Options, Include, State, or Definition\
        .");
      }
      // ItemType::Option => SectionItem::options_code(item),
      // ItemType::Include => SectionItem::Include(item),
      // ItemType::State => SectionItem::State(item)
    }
  }

  pub fn from_span(&self, code: Span) -> SectionItem {
    match self {
      ItemType::Top => SectionItem::User(code),
      ItemType::Class => SectionItem::Top(code),
      ItemType::Init => SectionItem::Class(code),
      ItemType::User => SectionItem::Init(code),
      ItemType::Unknown => SectionItem::Unknown(code),

      _ => {
        unreachable!("Cannot use ItemType::from_code to create an Options, Include, State, or \
        Definition.");
      }

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
      ItemType::Option => "%options",

      ItemType::State => {
        // This method is never called on `SectionItem::State`
        panic! {"SectionItem::State has multiple opening delimiters."};
      }
      ItemType::Definition => {
        // This method is never called on `SectionItem::Definition`
        panic! {"SectionItem::State has no opening delimiter."};
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
      | ItemType::Option
      | ItemType::Definition
      | ItemType::State => false,
    }
  }

  // For symmetry with `open_delimiter`
  pub fn close_delimiter(&self) -> &'static str {
    match self {
      | ItemType::Top
      | ItemType::Class
      | ItemType::Init
      | ItemType::Unknown => "}",

      ItemType::User => "%}",

      | ItemType::Include
      | ItemType::Option
      | ItemType::Definition
      | ItemType::State => ""
    }
  }
}

pub type SectionItemSet = Vec<SectionItem>;

#[derive(Clone, Debug)]
pub enum SectionItem {
  User(Span),
  Top(Span),
  Class(Span),
  Init(Span),
  Unknown(Span),
  Option(OptionField),
  Include {
    file: SourceFile<String, String>,
    contents: SectionItemSet,
  },
  State {
    is_exclusive: bool,
    name: Span,
  },
  Definition {
    name: Span,
    code: Span,
  },
}

impl Display for SectionItem {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    let text =
        match self {
          SectionItem::User(code) => {
            format!("User({:?})", code)
          }
          SectionItem::Top(code) => {
            format!("Top({:?})", code)
          }
          SectionItem::Class(code) => {
            format!("Class({:?})", code)
          }
          SectionItem::Init(code) => {
            format!("Init({:?})", code)
          }
          SectionItem::Unknown(code) => {
            format!("Unknown({:?})", code)
          }
          SectionItem::Include {
            file,
            contents,
          } => {
            format!("Include{{ file={:?} }}", file.name())
          }
          SectionItem::State {
            is_exclusive,
            name: code
          } => {
            format!("State{{is_exlusive: {:?}, code={:?} }}", is_exclusive, code)
          }
          SectionItem::Definition {
            name,
            code,
          } => {
            format!("Definition{{name: {:?}, regex={:?} }}", name, code)
          }
          SectionItem::Option(option) => {
            format!("Options: {:?}", *option)
          }
        };

    write!(f, "{}", text)
  }
}

impl SectionItem {
  pub fn user_code<S: ToSpan>(span: S) -> Self {
    SectionItem::User(span.to_span())
  }

  pub fn top_code<S: ToSpan>(span: S) -> Self {
    SectionItem::Top(span.to_span())
  }

  pub fn class_code<S: ToSpan>(span: S) -> Self {
    SectionItem::Class(span.to_span())
  }

  pub fn init_code<S: ToSpan>(span: S) -> Self {
    SectionItem::Init(span.to_span())
  }

  pub fn unknown_code<S: ToSpan>(span: S) -> Self {
    SectionItem::Unknown(span.to_span())
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
      SectionItem::Option(_) => ItemType::Option,
      SectionItem::State { .. } => ItemType::State,
      SectionItem::Definition { .. } => ItemType::Definition,
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

  pub fn get_code(&mut self) -> Option<&mut Span> {
    match self {
      | SectionItem::User(code)
      | SectionItem::Top(code)
      | SectionItem::Class(code)
      | SectionItem::Init(code)
      | SectionItem::State { name: code, .. }
      | SectionItem::Unknown(code) => Some(code),

      | SectionItem::Include { .. }
      | SectionItem::Definition { .. }
      | SectionItem::Option(_) => {
        None
      }
    }
  }
}


impl ToSpan for SectionItem {
  fn to_span(&self) -> Span {
    match self {
      | SectionItem::User(code)
      | SectionItem::Top(code)
      | SectionItem::Class(code)
      | SectionItem::Init(code)
      | SectionItem::State { name: code, .. }
      | SectionItem::Unknown(code) => *code,

      | SectionItem::Include { .. }
      | SectionItem::Definition { .. }
      | SectionItem::Option(_) => {
        panic!("Tried to turn {} into code.", self);
      }
    }
  }
}


impl Mergable for SectionItem {


  fn mergable(&self, other: &SectionItem) -> bool {

    if !self.is_code() || !other.is_code() {
      return false;
    }

    if self.item_type() != other.item_type() && !(self.item_type().is_code() &&
        other.item_type() == ItemType::Unknown) {
      return false;
    }

    self.to_span().mergable(&other.to_span())

  }


  /**
  Attempts tp merge `self` with `other`. This method is asymmetric: it assumes `self` was parsed
  before `other`.
  */
  fn merged<'a>(&'a mut self, other: &'a mut  SectionItem)
      -> Merged<&'a mut SectionItem, &'a mut SectionItem>
  {
    if self.item_type() == other.item_type() || self.item_type().is_code() && other.item_type()
        == ItemType::Unknown {
      match self {
        | SectionItem::User(self_code)
        | SectionItem::Top(self_code)
        | SectionItem::Class(self_code)
        | SectionItem::Init(self_code)
        | SectionItem::Unknown(self_code)
        => {
          // Unwrap always succeeds because of outer `if`.
          let other_code = other.get_code().unwrap();

          match self_code.merged(other_code) {
            Merged::Yes(_) => Merged::Yes(self),
            Merged::No(_, _) => Merged::No(self, other)
          }
        }

        | SectionItem::State{..}
        | SectionItem::Definition { .. }
        | SectionItem::Include{..}
        | SectionItem::Option(_) => Merged::No(self, other)

      } // end match self

    }
    else {
      Merged::No(self, other)
    }
  }
}
