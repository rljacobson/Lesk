#![feature(entry_insert)]

use nom_locate;
use lesk;



fn main() {
    //let _parser = Parser::new("(?imsqx)abc*|ghj", "bimopf=one.h, one.cpp, two.cpp, stdout;qrswx");
    //let _parser = Parser::new("abc*?|g{1,5}hj", "");

    let specification = Specification::default();
    println!("Options: {:?}", specification.options);
    println!("Done!")
}

