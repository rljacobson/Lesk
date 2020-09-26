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
use smallvec::SmallVec;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum ItemType {
  // SectionOne
  User,
  Top,
  Class,
  Init,
  Unknown,
  Include,
  State,
  Definition,
  Option,

  // Section Two
  ScannerTop,
  // Start,    //< Start States
}

impl Display for ItemType{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    let name =
        match self {
          // Section One
          ItemType::Top => "ItemType::Top",
          ItemType::Class => "ItemType::Class",
          ItemType::Init => "ItemType::Init",
          ItemType::User => "ItemType::User",
          ItemType::Unknown => "ItemType::Unknown",
          ItemType::Include => "ItemType::Include",
          ItemType::Option => "ItemType::Option",
          ItemType::State => "ItemType::State",
          ItemType::Definition => "ItemType::Definition",

          // Section Two
          ItemType::ScannerTop => "ItemType::ScannerTop",
        };

    write!(f, "{}", name)
  }
}

impl ItemType {
  pub fn new<S>(&self, item: S) -> Item
      where S: ToSpan
  {
    match self {
      ItemType::Top => Item::top_code(item.to_span()),
      ItemType::Class => Item::class_code(item.to_span()),
      ItemType::Init => Item::init_code(item.to_span()),
      ItemType::User => Item::user_code(item.to_span()),
      ItemType::Unknown => Item::unknown_code(item.to_span()),
      ItemType::ScannerTop => Item::scanner_top(item.to_span()),

      _ => {
        unreachable!("Cannot use ItemType::new to create an Options, Include, State, or Definition\
        .");
      }
      // ItemType::Option => SectionItem::options_code(item),
      // ItemType::Include => SectionItem::Include(item),
      // ItemType::State => SectionItem::State(item)
    }
  }

  pub fn open_delimiter(&self) -> &'static str {
    match self {
      ItemType::Top => "%top{",
      ItemType::Class => "%class{",
      ItemType::Init => "%init{",

      | ItemType::ScannerTop
      | ItemType::User => "%{",

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
      | ItemType::ScannerTop
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

      | ItemType::ScannerTop
      | ItemType::User => "%}",

      | ItemType::Include
      | ItemType::Option
      | ItemType::Definition
      | ItemType::State => ""
    }
  }
}

pub type SectionItemSet<'s> = SmallVec<[Item<'s>;1]>;

#[derive(Clone, Debug)]
pub enum Item<'s> {
  // Section One
  User(Span<'s>),
  Top(Span<'s>),
  Class(Span<'s>),
  Init(Span<'s>),
  Unknown(Span<'s>),
  Option(OptionField),
  Include {
    file: SourceFile<String, String>,
    contents: Vec<Item<'s>>,
  },
  State {
    is_exclusive: bool,
    name: Span<'s>,
  },
  Definition {
    name: Span<'s>,
    code: Span<'s>,
  },

  // Section Two
  ScannerTop(Span<'s>),
}

impl Display for Item {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    let text =
        match self {
          // Section One
          Item::User(code)    => format!("User({:?})",    code),
          Item::Top(code)     => format!("Top({:?})",     code),
          Item::Class(code)   => format!("Class({:?})",   code),
          Item::Init(code)    => format!("Init({:?})",    code),
          Item::Unknown(code) => format!("Unknown({:?})", code),


          Item::Include {
            file,
            contents,
          } => {
            format!("Include{{ file={:?} }}", file.name())
          }
          Item::State {
            is_exclusive,
            name: code
          } => {
            format!("State{{is_exlusive: {:?}, code={:?} }}", is_exclusive, code)
          }
          Item::Definition {
            name,
            code,
          } => {
            format!("Definition{{name: {:?}, regex={:?} }}", name, code)
          }
          Item::Option(option) => {
            format!("Options: {:?}", *option)
          }

          // Section Two
          Item::ScannerTop(code) => format!("ScannerTop({:?})", code),

        };

    write!(f, "{}", text)
  }
}

impl Item {
  pub fn user_code<S: ToSpan>(span: S) -> Self {
    Item::User(span.to_span())
  }

  pub fn top_code<S: ToSpan>(span: S) -> Self {
    Item::Top(span.to_span())
  }

  pub fn class_code<S: ToSpan>(span: S) -> Self {
    Item::Class(span.to_span())
  }

  pub fn init_code<S: ToSpan>(span: S) -> Self {
    Item::Init(span.to_span())
  }

  pub fn unknown_code<S: ToSpan>(span: S) -> Self {
    Item::Unknown(span.to_span())
  }

  pub fn scanner_top_code<S: ToSpan>(span: S) -> Self {
    Item::ScannerTop(span.to_span())
  }

  // Supplies the `ItemType` variant associated to this `SectionItem`.
  pub fn item_type(&self) -> ItemType {
    match self {
      // Section One
      Item::Top(_)            => ItemType::Top,
      Item::Class(_)          => ItemType::Class,
      Item::Init(_)           => ItemType::Init,
      Item::User(_)           => ItemType::User,
      Item::Unknown(_)        => ItemType::Unknown,
      Item::Include { .. }    => ItemType::Include,
      Item::Option(_)         => ItemType::Option,
      Item::State { .. }      => ItemType::State,
      Item::Definition { .. } => ItemType::Definition,

      // Section Two
      Item::ScannerTop(_) => ItemType::ScannerTop,
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
      | Item::User(code)
      | Item::Top(code)
      | Item::Class(code)
      | Item::Init(code)
      | Item::State { name: code, .. }
      | Item::ScannerTop(code)
      | Item::Unknown(code) => Some(code),

      | Item::Include { .. }
      | Item::Definition { .. }
      | Item::Option(_) => {
        None
      }
    }
  }
}


impl ToSpan for Item {
  fn to_span(&self) -> Span {
    match self {
      | Item::User(code)
      | Item::Top(code)
      | Item::Class(code)
      | Item::Init(code)
      | Item::State { name: code, .. }
      | Item::ScannerTop(code)
      | Item::Unknown(code) => *code,

      | Item::Include { .. }
      | Item::Definition { .. }
      | Item::Option(_) => {
        panic!("Tried to turn {} into code.", self);
      }
    }
  }
}


impl Mergable for Item {


  fn mergable(&self, other: &Item) -> bool {

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
  fn merged<'a>(&'a mut self, other: &'a mut Item)
      -> Merged<&'a mut Item, &'a mut Item>
  {
    if self.item_type() == other.item_type() || self.item_type().is_code() && other.item_type()
        == ItemType::Unknown {
      match self {
        | Item::Class(self_code)
        | Item::Init(self_code)
        | Item::Top(self_code)
        | Item::User(self_code)
        | Item::ScannerTop(self_code)
        | Item::Unknown(self_code)
        => {
          // Unwrap always succeeds because of outer `if`.
          let other_code = other.get_code().unwrap();

          match self_code.merged(other_code) {
            Merged::Yes(_) => Merged::Yes(self),
            Merged::No(_, _) => Merged::No(self, other)
          }
        }

        | Item::State{..}
        | Item::Definition { .. }
        | Item::Include{..}
        | Item::Option(_) => Merged::No(self, other)

      } // end match self

    }
    else {
      Merged::No(self, other)
    }
  }
}
