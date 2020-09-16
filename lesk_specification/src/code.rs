/*!
# A hierarchy of representations of code structures.
*/

use crate::Code;
use crate::options::OptionSet;
use crate::parser::{LSpan, ToSpan, Span};
use crate::section_items::{SectionOneItem, SectionOneItemSet};

use CodeBlock::*; // An enum defined below.


impl From<CodeBlock> for SectionOneItem {
  fn from(code_block: CodeBlock) -> Self {
    match code_block {
      CodeBlock::User(codes) => SectionOneItem::User(codes),
      CodeBlock::Top(codes) => SectionOneItem::Top(codes),
      CodeBlock::Class(codes) => SectionOneItem::Class(codes),
      CodeBlock::Init(codes) => SectionOneItem::Init(codes),
      CodeBlock::Unknown(codes) => SectionOneItem::Unknown(codes),
    }
  }
}

/*
impl SectionOneItem{
  pub fn merge(&mut self, rhs: SectionOneItem){
    if self.variant() != rhs.variant(){
      panic!("Cannot merge section item varians {} and {}.", self.variant(), rhs.variant());
    }

    match self {
      SectionOneItem::Code(pcb) => {
        let SectionOneItem::Code(other_pcb) = rhs;
        pcb.append(other_pcb);
      },
      SectionOneItem::Include {
        file,
        contents,
      },
      SectionOneItem::State {
        is_exclusive,
        code,
      },
      SectionOneItem::Options(os),
    }

  }

  pub fn variant(&self) -> SectionOneItemName {
    match self {
      SectionOneItem::Code(_) => SectionOneItemName::Code,
      SectionOneItem::Include{..} => SectionOneItemName::Include,
      SectionOneItem::State{..} => SectionOneItemName::State,
      SectionOneItem::Options(_) => SectionOneItemName::Options,
    }
  }
}

pub enum SectionOneItemName {
  Code,
  Include,
  State,
  Options,
}
*/

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

impl From<ParsedCode> for SectionOneItemSet {
  fn from(parsed_code: ParsedCode) -> Self {
    let mut result = SectionOneItemSet::default();

    if !parsed_code.unknown_code.is_empty() {
      result.push(Unknown(parsed_code.user_code).into());
    }
    if !parsed_code.unknown_code.is_empty() {
      result.push(Unknown(parsed_code.top_code).into());
    }
    if !parsed_code.unknown_code.is_empty() {
      result.push(Unknown(parsed_code.class_code).into());
    }
    if !parsed_code.unknown_code.is_empty() {
      result.push(Unknown(parsed_code.init_code).into());
    }
    if !parsed_code.unknown_code.is_empty() {
      result.push(Unknown(parsed_code.unknown_code).into());
    }

    result
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

  pub fn into_codes(self) -> Code {
    match self {
      | User(code)
      | Top(code)
      | Class(code)
      | Init(code)
      | Unknown(code) => code
    }
  }
}
