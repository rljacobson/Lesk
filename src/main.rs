#![feature(entry_insert)]


mod spec;

use structopt::StructOpt;

use spec::Specification;

fn main() {
    //let _parser = Parser::new("(?imsqx)abc*|ghj", "bimopf=one.h, one.cpp, two.cpp, stdout;qrswx");
    //let _parser = Parser::new("abc*?|g{1,5}hj", "");

    let specification = Specification::default();
    println!("Options: {:?}", specification.options);
    println!("Done!")
}

