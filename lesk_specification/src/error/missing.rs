//! Unexpected token error data structure.

use std::borrow::Cow;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

use codespan::{FileId, Span};
use codespan_reporting::diagnostic::{Diagnostic, Label};

use super::ToDiagnostic;
use crate::parser::ToSpan;

/// Error that occurs when an unexpected token was found
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissingError {
  /// Printable name of the token that is missing
  pub token: &'static str,
  /// Span of where the token was expected to be
  pub span: Span,
  /// An optional explanation of what is required
  pub explanation: Option<&'static str>,
}

impl MissingError {
  /// Constructs a new `MissingError`.
  pub fn new<S>(token: &'static str, span: S, explanation: Option<&'static str>) -> Self
    where
        S: ToSpan,
  {
    MissingError {
      token,
      span: span.to_span(),
      explanation
    }
  }
}

impl Display for MissingError {
  fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
    if let Some(explain) = self.explanation {
      write!(fmt, "missing: {}\n{}", self.token, explain)
    } else {
      write!(fmt, "missing: {}", self.token)
    }
  }
}

impl Error for MissingError {}

impl ToDiagnostic for MissingError {
  fn to_diagnostic(&self, file: FileId) -> Diagnostic<FileId> {
    let mut labels =
        vec![Label::primary(file, self.span).with_message("missing here")];

    Diagnostic::error().with_message(self.to_string()).with_labels(labels)
  }
}
