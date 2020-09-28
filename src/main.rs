#![feature(entry_insert)]

use lesk_specification::Specification;
use saucepan::Span;


fn main() {
  //let _parser = Parser::new("(?imsqx)abc*|ghj", "bimopf=one.h, one.cpp, two.cpp, stdout;qrswx");
  //let _parser = Parser::new("abc*?|g{1,5}hj", "");

  let mut specification = Specification::default();
  specification.parse();
  // println!("Options: {:?}", specification.options);
  println!("Done!")
}

