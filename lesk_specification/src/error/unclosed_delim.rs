/// Unclosed delimiters error data structure.
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

use codespan::{FileId, Span};
use codespan_reporting::diagnostic::{Diagnostic, Label};

use super::ToDiagnostic;
use crate::parser::ToSpan;

/// Error that occurs when at least one delimited span was left unclosed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnclosedDelimError {
    /// Location of open delimiter that lacks a matching close delimiter.
    pub unclosed_delimiter: Span,
    /// Span pointing to the end of the file.
    pub eof_span: Span,
}

impl UnclosedDelimError {
    /// Constructs a new `UnclosedDelimError`.
    pub fn new<S1, S2>(delim: S1, eof_span: S2) -> Self
    where
        S1: ToSpan,
        S2: ToSpan,
    {
        UnclosedDelimError {
            unclosed_delimiter: delim.to_span(),
            eof_span: eof_span.to_span(),
        }
    }
}

impl Display for UnclosedDelimError {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "this file contains un-closed delimiters")
    }
}

impl Error for UnclosedDelimError {}

impl ToDiagnostic for UnclosedDelimError {
    fn to_diagnostic(&self, file: FileId) -> Diagnostic<FileId> {
        let primary =
            Label::primary(file, self.eof_span).with_message("expected matching delimiter here");
        let mut diagnostic =
            Diagnostic::error().with_message(self.to_string()).with_labels(vec![primary]);

        let unclosed = Label::secondary(file, self.unclosed_delimiter).with_message("unmatched delimiter");
        diagnostic.labels.push(unclosed);

        diagnostic
    }
}
