/// Unclosed delimiters error data structure.
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

use codespan::{FileId, Span};
use codespan_reporting::diagnostic::{Diagnostic, Label};

use super::ToDiagnostic;
use crate::parser::ToSpan;

/// Error that occurs when `%%` is encountered inside a code block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnexpectedSectionEndError {
  /// Locations of open delimiters that lack a matching close delimiter.
  pub unclosed_delims: Vec<Span>,
  /// Span pointing to the section sentinel `%%` or to the EOF.
  pub end_span: Span,
}

impl UnexpectedSectionEndError {
  /// Constructs a new `UnexpectedSectionEnd`.
  pub fn new<S1, S2>(delims: Vec<S1>, eof_span: S2) -> Self
    where
        S1: ToSpan,
        S2: ToSpan,
  {
    UnexpectedSectionEndError {
      unclosed_delims: delims.into_iter().map(|span| span.to_span()).collect(),
      end_span: eof_span.to_span(),
    }
  }
}

impl Display for UnexpectedSectionEndError {
  fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
    write!(fmt, "section ending with unmatched delimiters")
  }
}

impl Error for UnexpectedSectionEndError {}

impl ToDiagnostic for UnexpectedSectionEndError {
  fn to_diagnostic(&self, file: FileId) -> Diagnostic<FileId> {


    let primary =
        Label::primary(file, self.end_span).with_message(
          "section ending encountered inside a code block with unmatched delimiter(s)"
        );
    let mut diagnostic =
        Diagnostic::error().with_message(self.to_string()).with_labels(vec![primary]);

    for span in &self.unclosed_delims {
      let unclosed = Label::secondary(file, *span).with_message("unmatched delimiter");
      diagnostic.labels.push(unclosed);
    }

    diagnostic
  }
}
