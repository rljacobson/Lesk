#![allow(dead_code)]

use super::position::Position;

#[macro_export]
macro_rules! debug_log {
    ($($args:expr),*) => {{
        $(
            print!("{}", $args);
        )*
    }}
}

// todo: Difference between `debug_log` and `debug_loga`?
#[macro_export]
macro_rules! debug_loga {
    ($($args:expr),*) => {{
        $(
            print!("{}", $args);
        )*
    }}
}

#[macro_export]
macro_rules! debug_logln {
    ($($args:expr),*) => {{
        $(
            print!("{}", $args);
        )*
        print!("{}", '\n');
    }}
}

pub fn debug_log_pos(p: Position){
  if p.is_accept() {
    debug_loga!(" ({})", p.accepts());
    if p.is_lazy() {
      debug_loga!("?{}", p.lazy());
    }
    if p.is_greedy() {
      debug_loga!("!");
    }
  }
  else {
    debug_loga!(" ");
    if p.is_iterable() {
      debug_loga!("{}.", p.iterations());
      debug_loga!("{}",  p.idx());
    }
    if p.is_lazy() {
      debug_loga!("?{}", p.lazy());
    }
    if p.is_anchor() {
      debug_loga!("^");
    }
    if p.is_greedy() {
      debug_loga!("!");
    }
    if p.is_ticked() {
      debug_loga!("'");
    }
  }
}
