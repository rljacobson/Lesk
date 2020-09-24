// todo: Remove this module in favor of `From` trait.
use nom_locate;
pub use codespan::Span;
use nom::{Slice, Offset};


pub type LSpan<'a> = nom_locate::LocatedSpan<&'a str>;

pub use crate::mergable::*;


pub trait HasSpan {
  fn span(&self) -> Span;
}

impl HasSpan for Span {
  fn span(&self) -> Span {
    *self
  }
}

pub trait ToSpan {
  fn to_span(&self) -> Span;
}

impl ToSpan for Span {
  fn to_span(&self) -> Span {
    *self
  }
}

impl<'a, T: ToSpan> ToSpan for &'a T {
  fn to_span(&self) -> Span {
    (*self).to_span()
  }
}

impl<'a> ToSpan for LSpan<'a> {
  fn to_span(&self) -> Span {
    let start = self.location_offset();
    let end = start + self.fragment().len();
    Span::new(start as u32, end as u32)
  }
}
