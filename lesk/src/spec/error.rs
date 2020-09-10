use codespan_reporting::diagnostic::{Diagnostic, Label, LabelStyle, Severity};

use super::*;
use span::*;

pub enum ErrorReport {
  UnmatchedBrace(Code),
}


