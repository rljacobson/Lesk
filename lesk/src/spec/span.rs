
use nom_locate;
use codespan::Span;



pub type LocatedSpan<'a> = nom_locate::LocatedSpan<&'a str>;
pub type PResult<'a, T> = nom::IResult<LocatedSpan<'a>, T>;


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

impl<'a> ToSpan for LocatedSpan<'a> {
  fn to_span(&self) -> Span {
    let start = self.offset;
    let end = start + self.fragment.len();
    Span::new(start as u32, end as u32)
  }
}
