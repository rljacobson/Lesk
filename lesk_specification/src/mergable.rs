/*!

A trait for structs that can determine if they can be merged with each other.

*/




use std::cmp::{min, max};
use std::fmt::{Display, Debug};
use smallvec::SmallVec;

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum Merged<T, U> {
  Yes(T),
  No(U, U)
}

impl<T, U> Merged<T, U> {
  pub fn unwrap(self) -> T {
    match self {

      Merged::Yes(t) => t,

      Merged::No(_, _) => {
        panic!("Attempted to unwrap a Merged::No(T, U).");
      }
    }
  }
}

pub trait Mergable {

  /*
    Determines whether a call to `Mergable::merged` will result in a `Merged::Yes(_)`. Two spans
    are mergable if their union is an interval.

    This is NOT `codespan::Span::disjoint()` (nor is it it's negation). Disjoint spans are
    mergable is they are adjacent: the intervals [a, b) and [b, c) are mergable but disjoint. .
  */
  fn mergable(&self, other: &Self) -> bool;

  /*›
    Attempts to merge self and other, producing `Merged::Yes(Self)` if successful and
    `Merged::No(Self, Self)` if failure. The result type is actually generic, so you can re-place
    `Self` in the results with, say, `&mut Self`.

    This operation is NOT `codespan::Span::merge()`, because `merged()` requires the spans to
    either overlap or to be adjacent (`first.end()==second.begin()`) and produces the union of
    the two spans. Neither are true of `codespan::Span::merge()`.
  */
  fn merged<'a>(&'a  mut self, other: &'a  mut Self) -> Merged<&'a mut Self, &'a mut Self>
    where Self: std::marker::Sized;

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

  fn merged<'a>(&'a mut self, other: &'a mut Self) -> Merged<&'a mut Self, &'a mut Self> {
    if self.mergable(&other){
      // The tablecloth trick: we pull `self` right out from beneath us.
      let mut merged_span = self.merge(*other);
      std::mem::swap(&mut merged_span, self);
      Merged::Yes(self)
    } else{
      Merged::No(self, other)
    }
  }

}

/*
impl Mergable for LocatedSpan<T>{


  fn mergable(&self, other: &Self) -> bool {
    // See `merged()` for `Span` for an explaination of why this works.
    let (first, second) = if self.location_offset() < other.location_offset() {
      (self, other)
    } else {
      (other, self)
    };

    second.start() <= first.location_offset() + first.len()
  }

  fn merged<'a>(&'a mut self, other: &'a mut Self) -> Merged<&'a mut Self, &'a mut Self> {
    // See `merged()` for `Span` for an explaination of why this works.
    if self.mergable(&other){
      // The tablecloth trick: we pull `self` right out from beneath us.
      self.fragment().spl


      let mut merged_span = self.merge(other);
      std::mem::swap(&mut merged_span, self);
      Merged::Yes(self)
    } else{
      Merged::No(self, other)
    }

  }

}

*/

pub fn merge_or_push_item<T, A>(items: &mut SmallVec<A>, mut item: T) -> &mut SmallVec<A>
  where T: Mergable + Display,
        A: smallvec::Array<Item=T>,
        // <A as smallvec::Array>::Item
{
  if items.is_empty() {
    // println!("Empty items while trying to merge with {}", item);
    items.push(item);
    return items;
  }

  // Unwrap always succeeds because of preceding `if`.
  let mut last_item = items.pop().unwrap();
  let mut result = last_item.merged(&mut item);

  match result {
    Merged::Yes(_) => {
      // println!("Success: {}", last_item);
      items.push(last_item);
    }

    Merged::No(_, _) => {
      // println!("failed.");
      items.push(last_item);
      items.push(item);
    }
  }

  items
}


pub fn merge_or_append_items<'a, T>(lhs: &'a mut Vec<T>, rhs: &'a mut Vec<T>) -> &'a mut Vec<T>
  where T: Mergable + Debug + Display
{
  if lhs.is_empty() {
    std::mem::swap(lhs, rhs);
    return lhs;
  } else if rhs.is_empty() {
    std::mem::swap(lhs, rhs);
    return lhs;
  }

  // Unwraps always succeed because of preceding `if` block.
  let mut lhs_last_item  = lhs.pop().unwrap();
  let mut rhs_first_item = rhs.first_mut().unwrap();
  match lhs_last_item.merged(rhs_first_item) {

    Merged::No(_, _) => {
      lhs.push(lhs_last_item); // Unpop
      // Can still "merge" the vectors
      lhs.append(rhs);
    }

    Merged::Yes(_) => {
      lhs.push(lhs_last_item);
      lhs.extend(rhs.drain(1..));
    }

  }
  lhs
}
