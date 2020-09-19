/*!

A trait for structs that can determine if they can be merged with each other.

*/




use codespan::Span;
use std::cmp::{min, max};
use nom_locate::LocatedSpan;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum Merged<T, U> {
  Yes(T),
  No(U, U)
}

pub trait Mergable {

  /*
    Determines whether a call to `Mergable::merged` will result in a `Merged::Yes(_)`. Two spans
    are mergable if their union is an interval.

    This is NOT `codespan::Span::disjoint()` (nor is it it's negation). Disjoint spans are
    mergable is they are adjacent: the intervals [a, b) and [b, c) are mergable but disjoint. .
  */
  fn mergable(&self, &other: Self) -> bool;

  /*
    Attempts to merge self and other, producing `Merged::Yes(Self)` if successful and
    `Merged::No(Self, Self)` if failure. The result type is actually generic, so you can re-place
    `Self` in the results with, say, `&mut Self`.

    This operation is NOT `codespan::Span::merge()`, because `merged()` requires the spans to
    either overlap or to be adjacent (`first.end()==second.begin()`) and produces the union of
    the two spans. Neither are true of `codespan::Span::merge()`.
  */
  fn merged<T, U>(&mut self, other: &mut Self) -> Merged<T, U>;

}


/// This `impl` does not check that the spans are from the same source.
impl Mergable for Span {
  fn mergable(&self, other: &Self) -> bool {
    // `first` is the span whose start is encountered first along the number line.
    let (first, second) = if self.start() <= other.start() {
      (self, other)
    } else {
      (other, self)
    };
    /*
    Two possibilities: either s2<=e1 or e1<s2. First case means mergable.
    Overlapping:
      s1         |          e1    |
      |         s2          |    e2
    Disjoint:
      s1            e1       |     |
      |             |       s2    e2
    */
    second.start() <= first.end()
  }

  fn merged(&mut self, other: &mut Self) -> Merged<Self, &mut Self> {
    if self.mergable(other){
      Merged::Yes(Span::new(min(self.start(), other.start()), max(self.end(), other.end())))
    } else{
      Merged::No(self, other);
    }
  }
}

