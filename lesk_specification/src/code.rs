#![allow(unused_code)]
/*!
# A hierarchy of representations of code structures.
*/

use std::fmt::Display;

use nom::lib::std::fmt::Formatter; // An enum defined below.

use crate::Code;
use crate::options::OptionSet;
use crate::parser::{
  LSpan,
  ToSpan,
  Span
};
use crate::section_items::{
  SectionItem,
  SectionItemSet
};

use CodeBlock::*;  // An enum defined below.

impl From<CodeBlock> for SectionItem {
  fn from(code_block: CodeBlock) -> Self {
    match code_block {
      CodeBlock::User(codes) => SectionItem::User(codes),
      CodeBlock::Top(codes) => SectionItem::Top(codes),
      CodeBlock::Class(codes) => SectionItem::Class(codes),
      CodeBlock::Init(codes) => SectionItem::Init(codes),
      CodeBlock::Unknown(codes) => SectionItem::Unknown(codes),
    }
  }
}

#[derive(Default, Clone, Debug)]
pub struct ParsedCode {
  pub user_code: Code,
  pub top_code: Code,
  pub class_code: Code,
  pub init_code: Code,
  pub unknown_code: Code,
}

impl From<CodeBlock> for ParsedCode {
  fn from(code_block: CodeBlock) -> Self {
    match code_block {
      User(mut codes) => {
        ParsedCode {
          user_code: codes,
          ..ParsedCode::default()
        }
      }

      Top(mut codes) => {
        ParsedCode {
          top_code: codes,
          ..ParsedCode::default()
        }
      }

      Class(mut codes) => {
        ParsedCode {
          class_code: codes,
          ..ParsedCode::default()
        }
      }

      Init(mut codes) => {
        ParsedCode {
          init_code: codes,
          ..ParsedCode::default()
        }
      }

      Unknown(mut codes) => {
        ParsedCode {
          unknown_code: codes,
          ..ParsedCode::default()
        }
      }
    }
  }
}


impl ParsedCode {
  pub fn append(&mut self, mut parsed_code_block: ParsedCode) -> &mut Self {
    self.user_code.append(&mut parsed_code_block.user_code);
    self.top_code.append(&mut parsed_code_block.top_code);
    self.class_code.append(&mut parsed_code_block.class_code);
    self.init_code.append(&mut parsed_code_block.init_code);

    self
  }

  pub fn push(&mut self, mut code_block: CodeBlock) -> &mut Self {
    match code_block {
      User(mut codes) => {
        self.user_code.append(&mut codes);
      }

      Top(mut codes) => {
        self.top_code.append(&mut codes);
      }

      Class(mut codes) => {
        self.class_code.append(&mut codes);
      }

      Init(mut codes) => {
        self.init_code.append(&mut codes);
      }
      Unknown(mut codes) => {
        self.unknown_code.append(&mut codes);
      }
    }
    self
  }
}

impl From<ParsedCode> for SectionItemSet {
  fn from(parsed_code: ParsedCode) -> Self {
    let mut result = SectionItemSet::default();

    if !parsed_code.user_code.is_empty() {
      result.push(User(parsed_code.user_code).into());
    }
    if !parsed_code.top_code.is_empty() {
      result.push(Top(parsed_code.top_code).into());
    }
    if !parsed_code.class_code.is_empty() {
      result.push(Class(parsed_code.class_code).into());
    }
    if !parsed_code.init_code.is_empty() {
      result.push(Init(parsed_code.init_code).into());
    }
    if !parsed_code.unknown_code.is_empty() {
      result.push(Unknown(parsed_code.unknown_code).into());
    }

    result
  }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum CodeBlockType {
  User,
  Top,
  Class,
  Init,
  Unknown,
}

impl Display for CodeBlockType{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    let name = match self {
      CodeBlockType::User => "User",
      CodeBlockType::Top => "Top",
      CodeBlockType::Class => "Class",
      CodeBlockType::Init => "Init",
      CodeBlockType::Unknown => "Unknown",
    };

    write!(f, "{}", name)
  }
}

impl CodeBlockType {
  pub fn open_delimiter(&self) -> &'static str{
    match self {
      CodeBlockType::Top => "%top{",
      CodeBlockType::Class => "%class{",
      CodeBlockType::Init => "%init{",
      CodeBlockType::User => "%{",
      CodeBlockType::Unknown => "{",
    }
  }

  // For symmetry with `open_delimiter`
  pub fn close_delimiter(&self) -> &'static str {
    "}"
  }
}


#[derive(Clone)]
pub enum CodeBlock {
  User(Code),
  Top(Code),
  Class(Code),
  Init(Code),
  Unknown(Code)
}

impl CodeBlock {
  pub fn user_code(span: Span) -> Self {
    User(vec![span])
  }

  pub fn top_code(span: Span) -> Self {
    Top(vec![span])
  }

  pub fn class_code(span: Span) -> Self {
    Class(vec![span])
  }

  pub fn init_code(span: Span) -> Self {
    Init(vec![span])
  }

  pub fn unknown_code(span: Span) -> Self {
    Unknown(vec![span])
  }

  pub fn push(&mut self, lspan: LSpan) {
    match self {
      | User(codes)
      | Top(codes)
      | Class(codes)
      | Init(codes)
      | Unknown(codes) => {
        codes.push(lspan.to_span());
      }
    }
  }

  pub fn into_code(self) -> Code {
    match self {
      | User(code)
      | Top(code)
      | Class(code)
      | Init(code)
      | Unknown(code) => code
    }
  }
}

