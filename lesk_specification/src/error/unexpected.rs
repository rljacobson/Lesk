//! Unexpected token error data structure.

use std::borrow::Cow;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

use codespan::Span;
use codespan_reporting::diagnostic::{Diagnostic, Label};

use super::{ToDiagnostic, FileId};
use crate::parser::ToSpan;

/// Error that occurs when an unexpected token was found.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnexpectedError {
  /// Printable name of the token that was found.
  pub token: &'static str,
  /// Span of the found token.
  pub span: Span,
  // Optional explanation
  pub explanation: Option<&'static str>
}

impl UnexpectedError {
  /// Constructs a new `UnexpectedError`.
  pub fn new<S>(token: &'static str, span: S, explanation: Option<&'static str>) -> Self
    where S: ToSpan,
  {
    UnexpectedError {
      token,
      span: span.to_span(),
      explanation
    }
  }
}

impl Display for UnexpectedError {
  fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
    if let Some(explain) = self.explanation {
      write!(fmt, "unexpected token: {}\n{}", self.token, explain)
    } else {
      write!(fmt, "unexpected token: {}", self.token)
    }
  }
}

impl Error for UnexpectedError {}

impl ToDiagnostic for UnexpectedError {
  fn to_diagnostic(&self, file: FileId) -> Diagnostic<FileId> {
    let labels =
        vec![Label::primary(file, self.span).with_message("found unexpected token here")];
    Diagnostic::error().with_message(self.to_string()).with_labels(labels)
  }
}
