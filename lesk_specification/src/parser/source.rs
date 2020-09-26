
use std::ffi::{OsStr, OsString};
use std::num::NonZeroU32;
use std::fmt::{Display, Formatter};

pub use saucepan::{
  ByteIndex,
  ColumnIndex,
  LineIndex,
  LineIndexOutOfBoundsError,
  LineOffset,
  Location,
  SourceID,
  SourceFiles,
  LocationError,
  RawIndex,
  Span as CodeSpan,
  SpanOutOfBoundsError,
};
use nom::Offset;


// use super::*;

pub type LSpan<'a> = nom_locate::LocatedSpan<&'a str>;


#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Span<'s>{
  pub source_id: SourceID,
  pub located_span: LSpan<'s>,
}




impl<'s> Span<'s>{

  /// Create a new span from a FileID and LSpan
  pub fn span_from_first(l_span: LSpan) -> Span{
    Span{
      source_id: SourceID(NonZeroU32(1)),
      located_span: l_span,
    }
  }

  /// Create a new span LSpan for the first (primary) `SourceID`.
  pub fn new(source_id: SourceID, located_span: LSpan) -> Span{
    Span{
      source_id,
      located_span
    }
  }

  pub fn len(&self) -> usize {
    self.fragment().len()
  }

  pub fn fragment(&self) -> &str {
    self.located_span.fragment()
  }

  pub fn start(&self) -> ByteIndex {
    self.located_span.offset() as ByteIndex
  }

  pub fn end(&self) -> ByteIndex {
    (self.located_span.offset() + self.fragment().len()) as ByteIndex
  }

}

impl<'a> From<LSpan<'a>> for Span<'a>{
  fn from(located_span: LSpan<'_>) -> Self {
    Span{
      source_id: SourceID(NonZeroU32(1)),
      located_span
    }
  }
}

impl<'s> AsRef<LSpan> for Span<'s>{
  fn as_ref(&self) -> &LSpan {
    &self.located_span
  }
}

impl<'s> From<Span<'s>> for CodeSpan{
  fn from(span: Span<'s>) -> Self {
    CodeSpan::new(span.start(), span.end())
  }
}
