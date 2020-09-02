#![feature(vec_remove_item)]
#![feature(new_uninit)]
#![feature(entry_insert)]
#![feature(map_first_last)]
#![feature(step_trait)]
#![feature(step_trait_ext)]

mod relesk;
mod valuecell;

use relesk::parser::Parser;

fn main() {
    //let _parser = Parser::new("(?imsqx)abc*|ghj", "bimopf=one.h, one.cpp, two.cpp, stdout;qrswx");
    let _parser = Parser::new("abc*?|g{1,5}hj", "");

    println!("Done!")
}
