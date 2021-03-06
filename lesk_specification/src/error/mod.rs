//! Error reporting data structures.

// todo: Factor out generic bits to a module at the library root level. Alternatively, collapse
// `*Error` structs into the `Error` enum.

mod expected_found;
mod incorrect_delim;
mod unclosed_delim;
mod unexpected;
mod invalid_label;
mod unexpected_section_end;
mod missing;

use std::borrow::Cow;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::iter::FromIterator;
use std::slice::Iter;

use codespan::{Span};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use nom::error::{ErrorKind, ParseError};
use smallvec::{smallvec, IntoIter, SmallVec};

pub use self::expected_found::ExpectedFoundError;
pub use self::incorrect_delim::IncorrectDelimError;
pub use self::unclosed_delim::UnclosedDelimError;
pub use self::unexpected::UnexpectedError;
pub use self::invalid_label::InvalidLabelError;
pub use self::unexpected_section_end::UnexpectedSectionEndError;
pub use self::missing::MissingError;

use crate::parser::ToSpan;
use super::SourceID;


// todo: refactor error framework to be more extensible.
// todo: update the examples


/// Trait for converting error types to pretty-printable diagnostics.
///
/// # Examples
///
/// ```
/// use codespan::{Files, Span};
/// use codespan_reporting::diagnostic::{Diagnostic, Label};
/// use error::ToDiagnostic;
///
/// struct MyError;
///
/// impl ToDiagnostic for MyError {
///     fn to_diagnostic(&self, file: SourceID) -> Diagnostic {
///         let label = Label::new(file, Span::new(2, 3), "error occurred here");
///         Diagnostic::new_error("something went wrong", label)
///     }
/// }
///
/// let mut files = Files::new();
/// let file_id = files.add("example.nix", "1 + 1");
///
/// let error = MyError;
/// println!("{:?}", error.to_diagnostic(file_id));
/// ```
pub trait ToDiagnostic {
    /// Converts this type to a [`Diagnostic`] using the given file ID.
    ///
    /// [`Diagnostic`]: https://docs.rs/codespan-reporting/0.5.0/codespan_reporting/diagnostic/struct.Diagnostic.html
    fn to_diagnostic(&self, file: SourceID) -> Diagnostic<SourceID>;
}

/// A growable stack for accumulating errors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Errors {
    pub errors: SmallVec<[Error; 1]>,
}

impl Errors {
    /// Constructs a new, empty `Errors` stack.
    ///
    /// The stack will not allocate until new errors are pushed onto it.
    ///
    /// # Examples
    ///
    /// ```
    /// # #![allow(unused_mut)]
    /// # use error::Errors;
    /// let mut errors = Errors::new();
    /// ```
    #[inline]
    pub fn new() -> Self {
        Errors {
            errors: SmallVec::new(),
        }
    }

    /// Returns the number of errors in the stack.
    ///
    /// # Examples
    ///
    /// ```
    /// # use error::Errors;
    /// let errors = Errors::new();
    /// assert_eq!(errors.len(), 0);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Returns `true` if the error stack is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use error::{Errors, UnexpectedError};
    /// use codespan::Span;
    ///
    /// let mut errors = Errors::new();
    /// assert!(errors.is_empty());
    ///
    /// errors.push(UnexpectedError::new("token", Span::new(3, 4)));
    /// assert!(!errors.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Appends a new error to the stack.
    ///
    /// # Examples
    ///
    /// ```
    /// # use error::{Errors, ExpectedFoundError};
    /// use codespan::Span;
    ///
    /// let mut errors = Errors::new();
    /// errors.push(ExpectedFoundError::new("foo", "bar", Span::new(0, 4)));
    /// assert_eq!(errors.len(), 1);
    /// ```
    #[inline]
    pub fn push<E>(&mut self, error: E)
    where
        E: Into<Error>,
    {
        self.errors.push(error.into());
    }

    /// Removes the last error from the stack and returns it, or [`None`] if it is empty.
    ///
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
    ///
    /// # Examples
    ///
    /// ```
    /// # use error::{Error, Errors, UnexpectedError};
    /// use codespan::Span;
    ///
    /// let mut errors = Errors::new();
    /// assert_eq!(errors.pop(), None);
    ///
    /// errors.push(UnexpectedError::new("token", Span::new(3, 4)));
    /// assert_eq!(errors.pop(), Some(Error::Unexpected(UnexpectedError::new("token", Span::new(3, 4)))));
    /// ```
    #[inline]
    pub fn pop(&mut self) -> Option<Error> {
        self.errors.pop()
    }

    /// Returns the last error in the stack, or [`None`] if it is empty.
    ///
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
    ///
    /// # Examples
    ///
    /// ```
    /// # use error::{Error, Errors, UnexpectedError};
    /// use codespan::Span;
    ///
    /// let mut empty = Errors::new();
    /// assert_eq!(empty.last(), None);
    ///
    /// let mut one = Errors::new();
    /// one.push(UnexpectedError::new("token", Span::new(3, 4)));
    /// assert_eq!(one.last(), Some(&Error::Unexpected(UnexpectedError::new("token", Span::new(3, 4)))));
    /// ```
    #[inline]
    pub fn last(&mut self) -> Option<&mut Error> {
        self.errors.last_mut()
    }

    /// Returns an iterator of errors.
    ///
    /// # Examples
    ///
    /// ```
    /// # use error::{Error, Errors, ExpectedFoundError, UnexpectedError};
    /// use codespan::Span;
    ///
    /// let mut errors = Errors::new();
    /// errors.push(UnexpectedError::new("token", Span::new(3, 4)));
    /// errors.push(ExpectedFoundError::new("foo", "bar", Span::new(0, 4)));
    ///
    /// let mut iter = errors.iter();
    /// assert_eq!(iter.next(), Some(&Error::Unexpected(UnexpectedError::new("token", Span::new(3, 4)))));
    /// assert_eq!(iter.next(), Some(&Error::ExpectedFound(ExpectedFoundError::new("foo", "bar", Span::new(0, 4)))));
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<Error> {
        self.errors.iter()
    }

    /// Converts each error to a new [`Diagnostic`] and collects them in a [`Vec`].
    ///
    /// [`Diagnostic`]: https://docs.rs/codespan-reporting/0.5.0/codespan_reporting/diagnostic/struct.Diagnostic.html
    /// [`Vec`]: https://doc.rust-lang.org/std/vec/struct.Vec.html
    ///
    /// # Examples
    ///
    /// ```
    /// # use error::{Errors, ExpectedFoundError};
    /// use codespan::{Files, SourceID, Span};
    ///
    /// let mut files = Files::new();
    /// let file_id = files.add("example.nix", "1 + 1");
    ///
    /// let mut errors = Errors::new();
    /// errors.push(ExpectedFoundError::new("-", "+", Span::new(2, 2)));
    ///
    /// let diagnostics = errors.to_diagnostics(file_id);
    /// println!("{:?}", diagnostics);
    /// ```
    pub fn to_diagnostics(&self, file: SourceID) -> Vec<Diagnostic<SourceID>> {
        self.errors.iter().map(|e| e.to_diagnostic(file)).collect()
    }
}

impl Default for Errors {
    /// Creates an empty `Errors` stack.
    #[inline]
    fn default() -> Self {
        Errors::new()
    }
}

impl Display for Errors {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        let mut errors = self.errors.iter().enumerate();

        if let Some((i, error)) = errors.next() {
            write!(fmt, "{}: {}", i, error)?;
        }

        for (i, error) in errors {
            write!(fmt, "\n{}: {}", i, error)?;
        }

        Ok(())
    }
}

impl std::error::Error for Errors {}

impl From<Error> for Errors {
    fn from(e: Error) -> Self {
        let mut errors = Errors::default();
        errors.push(e);
        errors
    }
}

impl Extend<Error> for Errors {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Error>,
    {
        self.errors.extend(iter);
    }
}

impl FromIterator<Error> for Errors {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Error>,
    {
        Errors {
            errors: SmallVec::from_iter(iter),
        }
    }
}

impl IntoIterator for Errors {
    type Item = Error;
    type IntoIter = IntoIter<[Self::Item; 1]>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}

impl<'a> IntoIterator for &'a Errors {
    type Item = &'a Error;
    type IntoIter = Iter<'a, Error>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.errors.iter()
    }
}

/// Implemented so `Errors` can be used as a custom error type in a [`nom::IResult`].
///
/// [`nom::IResult`]: https://docs.rs/nom/5.0.1/nom/type.IResult.html
impl<I> ParseError<I> for Errors
where
    I: ToSpan + ToString,
{
    fn from_error_kind(input: I, kind: ErrorKind) -> Self {
        Errors {
            errors: smallvec![Error::Nom(input.to_span(), kind)],
        }
    }

    fn append(input: I, kind: ErrorKind, mut other: Self) -> Self {
        if cfg!(debug_assertions) {
            other.push(Error::Nom(input.to_span(), kind));
        }
        other
    }
}

/// Kinds of errors that can accumulate in an [`Errors`] stack during parsing.
///
/// [`Errors`]: ./struct.Error.html
///
/// This error type implements [`ToDiagnostic`] so it can be easily converted to a pretty-printable
/// [`Diagnostic`].
///
/// [`ToDiagnostic`]: ./trait.ToDiagnostic.html
/// [`Diagnostic`]: https://docs.rs/codespan-reporting/0.5.0/codespan_reporting/diagnostic/struct.Diagnostic.html
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    /// A certain item was found, but was expecting something else.
    ExpectedFound(ExpectedFoundError),
    /// An incorrect closing delimiter was specified.
    IncorrectDelim(IncorrectDelimError),
    /// An invalid label was given for a code block.
    InvalidLabel(InvalidLabelError),
    /// At least one delimited span was left unclosed.
    UnclosedDelim(UnclosedDelimError),
    /// An unexpected token was found.
    Unexpected(UnexpectedError),
    /// An unexpected token was found.
    Missing(MissingError),
    /// The section ended inside a code block.
    UnexpectedSectionEnd(UnexpectedSectionEndError),
    /// A custom error with a span and message.
    Message(Span, Cow<'static, str>),
    /// A [`nom`] parse error occurred.
    ///
    /// [`nom`]: https://docs.rs/nom/5.0.1/nom/
    ///
    /// This kind of error may occur during parsing, but is expected to be discarded immediately
    /// once a successful path is found. Such errors should not normally be displayed to the user,
    /// as it indicates an unhandled case in the parser.
    Nom(Span, ErrorKind),
}

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        match *self {
            Error::ExpectedFound(ref e) => write!(fmt, "{}", e),
            Error::IncorrectDelim(ref e) => write!(fmt, "{}", e),
            Error::UnclosedDelim(ref e) => write!(fmt, "{}", e),
            Error::Unexpected(ref e) => write!(fmt, "{}", e),
            Error::Missing(ref e) => write!(fmt, "{}", e),
            Error::UnexpectedSectionEnd(ref e) => write!(fmt, "{}", e),
            Error::InvalidLabel(ref e) => write!(fmt, "{}", e),
            Error::Message(_, ref e) => write!(fmt, "{}", e),
            Error::Nom(_, ref e) => write!(fmt, "nom error: {:?}", e),
        }
    }
}

impl std::error::Error for Error {}

impl From<ExpectedFoundError> for Error {
    fn from(error: ExpectedFoundError) -> Self {
        Error::ExpectedFound(error)
    }
}

impl From<IncorrectDelimError> for Error {
    fn from(error: IncorrectDelimError) -> Self {
        Error::IncorrectDelim(error)
    }
}

impl From<InvalidLabelError> for Error {
    fn from(error: InvalidLabelError) -> Self {
        Error::InvalidLabel(error)
    }
}

impl From<UnclosedDelimError> for Error {
    fn from(error: UnclosedDelimError) -> Self {
        Error::UnclosedDelim(error)
    }
}

impl From<UnexpectedError> for Error {
    fn from(error: UnexpectedError) -> Self {
        Error::Unexpected(error)
    }
}

impl From<UnexpectedSectionEndError> for Error {
    fn from(error: UnexpectedSectionEndError) -> Self {
        Error::UnexpectedSectionEnd(error)
    }
}

impl ToDiagnostic for Error {
    fn to_diagnostic(&self, file: SourceID) -> Diagnostic<SourceID> {
        match *self {
            Error::ExpectedFound(ref e) => e.to_diagnostic(file),
            Error::IncorrectDelim(ref e) => e.to_diagnostic(file),
            Error::InvalidLabel(ref e) => e.to_diagnostic(file),
            Error::UnclosedDelim(ref e) => e.to_diagnostic(file),
            Error::Unexpected(ref e) => e.to_diagnostic(file),
            Error::Missing(ref e) => e.to_diagnostic(file),
            Error::UnexpectedSectionEnd(ref e) => e.to_diagnostic(file),
            Error::Message(ref span, ref msg) => {
                let label = Label::primary(file, *span).with_message(msg.clone());

                Diagnostic::error().with_message(msg.clone())
                                   .with_labels(vec![label])
            }
            Error::Nom(ref span, ref kind) => {
                let label = Label::primary(file, *span).with_message(self.to_string());
                let note =
                    "note: this indicates an unhandled case in the parser".to_string();

                Diagnostic::bug().with_message(format!("nom error: {:?}", kind))
                                 .with_labels(vec![label])
                                 .with_notes(vec![note])
            }

        }
    }
}
