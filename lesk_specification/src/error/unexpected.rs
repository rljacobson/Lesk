//! Unexpected token error data structure.

use std::borrow::Cow;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

use codespan::{FileId, Span};
use codespan_reporting::diagnostic::{Diagnostic, Label};

use super::ToDiagnostic;
use crate::parser::ToSpan;

/// Error that occurs when an unexpected token was found.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnexpectedError {
    /// Printable name of the token that was found.
    pub token: Cow<'static, str>,
    /// Span of the found token.
    pub span: Span,
}

impl UnexpectedError {
    /// Constructs a new `UnexpectedError`.
    pub fn new<T, S>(token: T, span: S) -> Self
    where
        T: Into<Cow<'static, str>>,
        S: ToSpan,
    {
        UnexpectedError {
            token: token.into(),
            span: span.to_span(),
        }
    }
}

impl Display for UnexpectedError {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        write!(fmt, "unexpected token: {}", self.token)
    }
}

impl Error for UnexpectedError {}

impl ToDiagnostic for UnexpectedError {
    fn to_diagnostic(&self, file: FileId) -> Diagnostic<FileId> {
        let label =
            Label::primary(file, self.span).with_message( "found unexpected token here");
        Diagnostic::error().with_message(self.to_string()).with_labels(vec![label])
    }
}
