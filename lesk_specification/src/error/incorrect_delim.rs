//! Incorrect closing delimiter error data structure.

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

use codespan::Span;
use codespan_reporting::diagnostic::{Diagnostic, Label};

use super::{ToDiagnostic, SourceID};
use crate::parser::ToSpan;

/// Error that occurs when an incorrect closing delimiter was specified.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncorrectDelimError {
  /// The unmatched delimiter and its location in the source file.
  pub unmatched_delim: (&'static str, Span),
  /// Location where a possible closing delimiter could be placed.
  pub candidate_span: Option<Span>,
  /// Span from the unmatched character to EOF.
  pub unclosed_span: Span,
}

impl IncorrectDelimError {
  /// Constructs a new `IncorrectDelimError`.
  pub fn new<S, U>(delim: U, span: S, candidate: Option<S>, unclosed: S) -> Self
    where S: ToSpan,
          U: Into<&'static str>
  {
    IncorrectDelimError {
      unmatched_delim: (delim.into(), span.to_span()),
      candidate_span: candidate.map(|s| s.to_span()),
      unclosed_span: unclosed.to_span(),
    }
  }
}

impl Display for IncorrectDelimError {
  fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
    write!(
      fmt,
      "incorrect close delimiter: `{}`",
      self.unmatched_delim.0
    )
  }
}

impl Error for IncorrectDelimError {}

impl ToDiagnostic for IncorrectDelimError {
  fn to_diagnostic(&self, file: SourceID) -> Diagnostic<SourceID> {
    let primary =
        Label::primary(file, self.unmatched_delim.1).with_message("incorrect close delimiter");
    let mut diagnostic = Diagnostic::error().with_message(self.to_string())
                                                              .with_labels(vec![primary]);

    if let Some(span) = self.candidate_span {
      let candidate =
          Label::secondary(file, span).with_message("close delimiter possibly meant for this");
      diagnostic.labels.push(candidate);
    }

    let unclosed = Label::secondary(file, self.unclosed_span).with_message("unmatched delimiter");
    diagnostic.labels.push(unclosed);

    diagnostic
  }
}
