//! Incorrect closing delimiter error data structure.

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

use codespan::Span;
use codespan_reporting::diagnostic::{Diagnostic, Label};

use super::{ToDiagnostic, SourceID};
use crate::parser::{ToSpan, LSpan};

/// Error that occurs when an incorrect closing delimiter was specified.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvalidLabelError {
  /// The unmatched delimiter and its location in the source file.
  pub label: (Span, Span),
  /// Location where a possible closing delimiter could be placed.
  pub code_block: Option<Span>,
}

impl<'a> InvalidLabelError {
  /// Constructs a new `IncorrectDelimError`.
  pub fn new<S, U>(text: U, span: S, code_block: Option<S>) -> Self
    where S: ToSpan,
          U: ToSpan,
  {
    InvalidLabelError {
      label: (text.to_span(), span.to_span()),
      code_block: code_block.map(|s| s.to_span())
    }
  }
}

impl Display for InvalidLabelError {
  fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
    write!(
      fmt,
      "invalid code block label: `{}`",
      self.label.0
    )
  }
}

impl Error for InvalidLabelError {}

impl ToDiagnostic for InvalidLabelError {
  fn to_diagnostic(&self, file: SourceID) -> Diagnostic<SourceID> {
    let primary =
        Label::primary(file, self.label.1).with_message("invalid code block label");
    let mut diagnostic =
        Diagnostic::error().with_message(self.to_string()).with_labels(vec![primary]);

    Option::and_then::<Span, _>(self.code_block, |cb_span| {
      let label = Label::secondary(file, cb_span)
          .with_message("this code block needs a valid label");
      diagnostic.labels.push(label);
      None // Return value unused.
    });

    diagnostic
  }
}
