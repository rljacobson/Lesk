//! Expected/found error data structure.

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

use codespan::Span;
use codespan_reporting::diagnostic::{Diagnostic, Label};

use super::ToDiagnostic;
use crate::SourceID;

/// Error that occurs when an item was found, but was expecting something else.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedFoundError {
    /// Printable name of the item that was expected.
    pub expected: &'static str,
    /// Printable name of the item that was found.
    pub found: &'static str,
    /// Span of the found item.
    pub span: Span,
}

impl ExpectedFoundError {
    /// Constructs a new `ExpectedFoundError`.
    pub fn new<T, U, S>(expected: T, found: U, span: S) -> Self
    where
        T: Into<&'static str>,
        U: Into<&'static str>,
        S: ToSpan,
    {
        ExpectedFoundError {
            expected: expected.into(),
            found: found.into(),
            span: span.to_span(),
        }
    }
}

impl Display for ExpectedFoundError {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "expected {}, found {}", self.expected, self.found)
    }
}

impl Error for ExpectedFoundError {}

impl ToDiagnostic for ExpectedFoundError {
    fn to_diagnostic(&self, file: SourceID) -> Diagnostic<SourceID> {
        let label = Label::primary(file, self.span)
                        .with_message(format!("expected {} here", self.expected));
        Diagnostic::error().with_message(self.to_string()).with_labels(vec![label])
    }
}
