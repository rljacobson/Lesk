#![allow(dead_code)]


use std::cmp::max;
use std::time::Duration;

use defaultmap::DefaultHashMap;
use quanta::Clock;
use crate::valuecell::ValueCell;

use super::*;
use error::RegexError;
use group::{Group, TargetSet};
use limits::{MAX_INDEX, MAX_ITER};
use modifier::Mode;
use modifier::Modifiers;
use options::Options;
use chars;
use crate::relesk::chars::{POSIX_CLASS_NAMES, POSIX_CLASSES};


static END_ESCAPE_OPTION : &[u8; 39] = b"0123456789<>ABDHLNPSUWXbcdehijklpsuwxz\0";
static META_OPS          : &[u8;  9] = b".^$([{?*+";
static CODE_EXTENSIONS   : [&str; 4] = [".h", ".hpp", ".cpp", ".cc"];
static DFA_EXTENSIONS    : [&str; 1] = [".gv"];

#[derive(Default)]
pub struct Parser<'a> {
  idx       : Index32,   //< Cursor into `self.regex`
  modifiers : Modifiers, //< Describes which modifiers are active at which positions in the regex
  options   : Options,   //< Pattern compiler options
  regex     : &'a [u8],  //< Regular expression string as bytes

  group          : ValueCell<Group>, //< The outermost matching group representing the entire regex
  next_group_idx : Index32,          //< A "global" variable keeping track of the index for the next new group
  is_first_group : bool,             //< Only true while parsing the outer-most group



  // Shared?
  pub lazy_set: HashSet<Lazy8>, //< Positions in the regex that are lazily matched.

  /// For each position in the group, which positions can follow it.
  pub follow_positions_map : FollowMap,
  pub start_positions      : PositionSet, //< Accumulates first positions


  /**
  Maps a top-level group index to the set of lookaheads for that index. The keys are `Index32`s, and
  the values are `PositionSet`s. The `lookahead` `PositionSet` for a subgroup is obtained by using
  `group.idx` as a key.

  Subgroups are Consulted in `parse_iterated` (parse2) and filled in `parse_alternations` (parse4).
  */
  pub lookahead_map: DefaultHashMap<Index32, IndexRanges>,

  //Timing
  parse_time: Duration
}

impl<'a> Parser<'a> {

  pub fn new<'p>(regex: &'p str, options_string: &'p str) -> Parser<'p>{
    let mut parser: Parser = Parser::default();
    parser.options.parse_options(options_string);

    parser.regex = regex.as_bytes();

    parser.build();

    parser
  }


  pub fn build(&mut self) {
    // Parse the regex pattern to construct the `self.follow_positions_map` NFA without epsilon
    // transitions
    self.parse();

    // start state = start_positions = first_position of the self.follow_positions_map NFA, also
    // merge the tree DFA root when non-std::ptr::null()=
    //self.start = State::new();
    // Compile the NFA into a DFA
    //self.compile();
    // Assemble DFA opcode tables or direct code
    //self.assemble();
  }

  // region Inlined Methods

  /// Returns the character at the index `idx` of the regular expression.
  #[must_use]
  fn at(&self, idx: Index32) -> Char {
    if idx >= self.regex.len() as Index32 {
      // We do not return `Option<Char>` to keep unwrapping to reasonable levels.
      return '\0'.into();
    }
    Char::from(self.regex[idx as usize])
  }

  /// Same as `at()` but assumes `idx=self.idx`.
  #[must_use]
  fn c(&self) -> Char{
    self.at(self.idx)
  }

  /// Same as `c()` but post-increments `self.idx`.
  #[must_use]
  fn ci(&mut self) -> Char{
    self.idx += 1;
    self.at(self.idx-1)
  }


  /// Same as `c()` but PRE-increments `self.idx`.
  #[must_use]
  fn cr(&mut self) -> Char{
    self.idx += 1;
    self.at(self.idx)
  }

  fn next_group_index(&mut self) -> Index32 {
    self.next_group_idx =
    self.next_group_idx.checked_add(1).unwrap_or_else(
      |   | {
        // overflow: too many top-level alternations (should never happen)
        RegexError::ExceedsLimits(self.idx).emit();
        //unreachable!("Unreachable: Error should have panicked but didn't.")
      }
    );

    self.next_group_idx
  }


  #[must_use]
  pub fn eq_at(&self, loc: Index32, s: &str) -> bool {
    return self.regex[loc as usize..].starts_with(s.as_bytes());
  }

  #[must_use]
  pub fn escape_at(&self, loc: Index32) -> Option<Char> {
    if self.at(loc) == self.options.escape_character {
      return Some(self.at(loc + 1));
    }
    None
  }

  #[must_use]
  pub fn escapes_at(&self, loc: Index32, escapes: &[u8]) -> Option<Char> {
    if self.at(loc) == self.options.escape_character &&
    escapes.contains(&(self.at(loc + 1).0 as u8))
    {
      return Some(self.at(loc + 1));
    }
    // return '\0' as Char;
    None
  }


  /**
    Searches for the first occurrence of `c` in `self.regex` starting at `loc`, returning its index.
  */
  pub fn find_at(&self, idx: Index32, c: char) -> Option<Index32> {
    self.regex[(idx as usize)..]
    .iter()
    .position(| &x | x == (c as u8) )
    .and_then(|index| Some(idx + (index as Index32)))
  }

  


  // endregion


  /**
  ## Stage 0A
  The top-level `parse` function parses modifiers of the form `(?imqsx-imqsx)`, alternations, and
  string literal patterns and then calls `parse_anchors` to parse its subpatterns.

  Only called once, and calls `parse_anchors`. Recursive calls are to `parse_alternations`.

  ```

  parse ⟶ parse_anchors ⟶ parse_iterated ⟶ parse_sequence
            ↑                                                │
            └────────────── parse_alternations ←─────────────┘

  ```
  */
  fn parse(&mut self) {
    let len: Index32 = self.regex.len() as Index32;

    println!("BEGIN parse() <parse0A>");

    if len > MAX_INDEX {
      RegexError::ExceedsLength(MAX_INDEX).emit();
    }

    // Timing
    let timer: Clock = Clock::new();
    let parse_start_time = timer.start();

    self.parse_global_modifiers();


    self.is_first_group = true;


    // Parse each subpattern.
    loop {

      // We first look for a string literal pattern.
      let mut end: Index32 = self.idx;
      if !self.options.quote_with_x && !self.options.x_freespacing {
        // TODO: perhaps allow \< \> and ^ $ anchors with string patterns?
        loop {
          let mut c: Char = self.at(end);

          if c == '\0' || c == '|' {
            break;
          }

          if META_OPS.contains(&c.into()) {
            end = self.idx;
            break;
          }

          if c == self.options.escape_character {
            end += 1;
            c = self.at(end);
            if END_ESCAPE_OPTION.contains(&c.into()) {
              end = self.idx;
              break;
            }
            // Flex/Lex style quotations: `\Q...\E`
            if c == 'Q' {
              loop {
                end += 1;
                c = self.at(end);
                if c == '\0' {
                  break;
                }
                if c == self.options.escape_character && self.at(end + 1) == 'E' {
                  break;
                }
              }
            }
          }
          end += 1;
        }
      }

      if self.idx < end {
        // String pattern found (w/o regex metas). Insert string into the string literal trie.
        let mut string_literal = String::new();

        let mut quote: bool = false;
        while self.idx < end {
          let mut c: Char = self.c();
          self.idx += 1;
          if c == self.options.escape_character {
            if self.c() == 'Q' {
              quote = true;
              self.idx += 1;
              continue;
            }
            if self.c() == 'E' {
              quote = false;
              self.idx += 1;
              continue;
            }
            if !quote {
              self.idx += 1;
              c = self.c();
              // If it's an escape character, convert to the ASCII character it refers to.
              if let Some(descaped_c) = Char::try_from_escape(c) {
                c = descaped_c
              }
            }
          }
          else if c >= 'A' && c <= 'Z' && self.options.insensitive_case {
            c = c.to_lowercase();
          }
          string_literal.push(c.into());
        }
        let next_idx = self.next_group_index();
        self.group.borrow_mut().insert_string(&string_literal, next_idx);

      }
      else {
        let group = self.group.clone();
        let mut group_ref = group.borrow_mut();

        self.parse_anchors(&mut group_ref);

        group_ref.subpattern_endpoints.push(self.idx);
        self.start_positions.append(&mut group_ref.first_positions);
        group_ref.append_idx_as_lazy_accepted(&mut self.start_positions);
        group_ref.append_idx_for_last_positions(&mut self.follow_positions_map);
      }

      if self.c() != '|' {
        self.idx += 1;
        break;
      }
      self.idx += 1;
    }

    if self.options.insensitive_case {
      self.modifiers.set(Mode::i, 0..len - 1)
    }

    if self.options.multiline {
      self.modifiers.set(Mode::m, 0..len - 1)
    }

    if self.options.single_line {
      self.modifiers.set(Mode::s, 0..len - 1)
    }

    self.parse_time = timer.delta(parse_start_time, timer.end());

    #[cfg(feature = "DEBUG")]
    {
      print!("start_positions = {{");
      self.debug_log_position_set(&self.start_positions, 0);
      println!("}}");
      self.debug_log_follow_map(0);
    }
    println!("END parse()");
    println!("Duration: {}", self.parse_time.as_millis());
  }


  /**
  ## Stage 0B
  Parse "multiple modifiers mode," e.g. `(?imsux-imsux:φ)`, where the modifiers before the dash are
  enabled and the mode modifiers after the dash are disabled.
  */
  fn parse_global_modifiers(&mut self){
    println!("BEGIN parse_global_modifiers() <parse0B>");

    if self.c() == '(' && self.at(1) == '?' {
      self.idx = 2;
      print!("(?");

      // Compute the ending location of the option expression.
      while self.c().is_alphanumeric() || self.c() == '-' {
        self.idx += 1;
      }

      if self.c() == ')' {
        let mut active: bool = true;
        self.idx = 2;

        let mut c: char = self.c().into();
        while c != ')' {
          print!("{}", c);
          match c {
            '-' => {
              active = false;
            }
            'i' => {
              self.options.insensitive_case = active;
            }
            'm' => {
              self.options.multiline = active;
            }
            'q' => {
              self.options.quote_with_x = active;
            }
            's' => {
              self.options.single_line = active;
            }
            'x' => {
              self.options.x_freespacing = active;
            }
            _ => {
              RegexError::InvalidModifier(self.idx).emit();
            }
          }
          self.idx += 1;
          c = self.c().into();
        }
        println!(")");
        // Skip the ')'
        self.idx += 1;
      } // end if c == ')'
      else {

        // The case that `(?...)` contains more than just options.. Resetting this index effectively
        // reinterprets the expression as a match group.

        self.idx = 0;
      }
    }
  }


  /**
  ## Stage 1
  Parses alternations.

  Only ever called from parse_regex4, which clears the global vars first. parse_regex2 inserts
  elements into the global vars. parse_regex3 calls regex_parse4, also clears first, last, and lazy.
  */
  fn parse_alternations(&mut self, group: &mut Group){
    println!("BEGIN parse_alternations({})", self.idx);

    // Called with provided group
    self.parse_anchors(group);
    
    let mut new_group = Group::default();
    new_group.idx = self.next_group_index();
    new_group.lazy_index = group.lazy_index;

    while self.ci() == '|' {

      self.parse_anchors(&mut new_group);
      // Update the old values.
      group.first_positions.extend(new_group.first_positions.iter());
      group.last_positions.extend(new_group.last_positions.iter());
      group.lazy_set.extend(new_group.lazy_set.iter());

      
      group.nullable   = new_group.nullable || group.nullable;
      group.iteration = max(new_group.iteration, group.iteration);
    }

    println!("END parse_alternations");
  }


  /**
  ## Stage 2
  Parses anchored groups
  */
  fn parse_anchors(&mut self, group: &mut Group){
    
    println!("BEGIN parse_anchors({}) <parse2>", self.idx);

    let mut anchor_positions: PositionSet = PositionSet::default();
    if self.is_first_group {
      loop {

        if self.options.x_freespacing {
          while self.c().is_whitespace() { self.idx += 1; }
        }

        // Check for BOL anchor
        if self.c() == '^' {
          anchor_positions.insert(Position(self.idx as u64));
          self.idx += 1;
          self.is_first_group = false; // CHECKED algorithmic options: 7/29 but does not allow ^ as a pattern
        }
        /*
          \A: begin of input
          \B: starting at non-word boundary
          \b: starting at word boundary
          \<: starts a word
          \>: starts a non-word
        */
        else if self.escapes_at(self.idx, b"ABb<>").is_some() {
          anchor_positions.insert(Position(self.idx as u64));
          self.idx += 2;
          self.is_first_group = false; // CHECKED algorithmic options: 7/29 but does not allow \b as a pattern
        }
        /*
          \i: matches an indent
          \j: matches a dedent
        */
        else if self.escapes_at(self.idx, b"ij").is_some(){
          self.is_first_group = false;
          break;
        }
        else {
          break;
        }

      }
    }

    self.parse_iterated(group);

    let mut new_group = Group::default();
    new_group.idx = self.next_group_index();
    new_group.lazy_index = group.lazy_index;

    let mut c: Char = self.c();
    while c != '\0' && c != '|' && c != ')' {
      self.parse_iterated(&mut new_group);
      //self.parse_regex3(
      //  false,
        // self.idx,

        // &firstpos1,
        // &lastpos1,
        // nullable1,
        // &lazyset1,
        // iter1

        // self.follow_positions_map,
        // group.lazy_index,
        // modifiers,
        // lookahead,
      //);

      if !group.lazy_set.is_empty(){
        /*
          CHECKED self is an extra rule for + only and (may) not be needed for *
          CHECKED algorithmic options: self.lazy(firstpos1); does not work for (a|b)*?a*b+,
          below works
        */
        group.extend_with_lazy(TargetSet::First, &new_group.first_positions)
        /*
          if (lazyset1.is_empty())
          greedy(firstpos1); // CHECKED algorithmic options: 8/1 works except fails for
            ((a|b)*?b){2} and (a|b)??(a|b)??aa
        */
      }


      for p in group.last_positions.iter() {
        self.follow_positions_map.get_mut(p.idx().into()).extend(new_group.first_positions.iter());
      }

      if group.nullable {

        group.extend_with(TargetSet::First, &new_group.first_positions);
        group.extend_with(TargetSet::Last, &new_group.last_positions);
        group.extend_with(TargetSet::Lazy, &new_group.lazy_set); // CHECKED 10/21
      }
      else {
        std::mem::swap(&mut group.last_positions, &mut new_group.last_positions);
        std::mem::swap(&mut group.lazy_set, &mut new_group.lazy_set);
        group.nullable = false;
      }

      // CHECKED 10/21 set_insert(self.lazy_set, lazyset1);
      group.iteration = max(new_group.iteration, group.iteration);
      c = self.c();
    }


    for p in anchor_positions.iter() {
      for k in group.last_positions.iter() {

        if self.at(k.idx()) == ')'
          // todo: Can group.idx be trusted to give the right `lookahead` set?
          && self.lookahead_map[group.idx].contains(&k.idx())
        {
          self.follow_positions_map.get_mut(p.idx().into()).insert(*k);
        }

        self.follow_positions_map.get_mut(k.idx().into()).insert(
          p.set_anchor( !group.nullable || k.idx() != p.idx() )
        );
      }

      group.last_positions.clear();
      group.last_positions.insert(*p);

      if group.nullable {
        group.first_positions.insert(*p);
        group.nullable = false;
      }
    }
    println!("END parse_anchors()");
  }



  /**
  ## Stage 3
  Parses repeated/optional subexpressions: `*`, `+`, `?`
  */
  fn parse_iterated(&mut self, group: &mut Group) {
    println!("BEGIN parse_iterated({}) <parse3>", self.idx);

    let original_position: Position = Position(self.idx.into());

    // Called with original global values
    self.parse_sequence(group);
    //self.parse_regex4(
    //  begin,
      // self.idx,
      // first_positions,
      // last_positions,
      // nullable,
      // follow_positions_map,
      // lazy_index,
      // lazy_set,
      // modifiers,
      // lookahead,
      // iteration
    //);

    let mut c: Char = self.c();
    if self.options.x_freespacing {
      while c.is_whitespace() {
        c = self.ci();
      }
    }

    loop {
      if c == '*' || c == '+' || c == '?' {
        if c == '*' || c == '?' {
          group.nullable = true;
        }
        self.idx += 1;
        if self.c() == '?' {

          group.increment_lazy_index();

          self.lazy_set.insert(group.lazy_index); // overflow: exceeds max 255 lazy quantifiers
          if group.nullable {
            group.lazify_own_set(TargetSet::First);
          }
          self.idx += 1;
        } else {
          // CHECKED algorithmic options: 7/30 if !group.nullable {
          // CHECKED algorithmic options: 7/30   self.lazy_set.clear();
          group.first_positions = greedify(&group.first_positions);
        }
        if c == '+' && !group.nullable && !self.lazy_set.is_empty() {
          let more_first_positions: PositionSet = group.lazify(&group.first_positions);
          for p in group.last_positions.iter() {
            self.follow_positions_map[p.idx().into()].extend(more_first_positions.iter());
          }
          group.first_positions.extend(more_first_positions.iter());
        }
        else if c == '*' || c == '+' {
          for p in group.last_positions.iter() {
            self.follow_positions_map[p.idx().into()].extend(group.first_positions.iter());
          }
        }
      }
      else if c == '{' {
        // {n,m} repeat min n times to max m

        let k = self.parse_digit();
        if k > MAX_ITER as usize {
          RegexError::ExceedsLimits(self.idx).emit();
        }

        let n: Iteration16 = k as Iteration16;
        let mut m: Iteration16 = n;
        let mut unlimited: bool = false;

        if self.c() == ',' {
          if self.at(self.idx + 1).is_digit() {
            m = self.parse_digit() as Iteration16;
          } else {
            unlimited = true;
            self.idx += 1;
          }
        }

        if self.c() == '}' {
          // todo: Is nullable1 (now old_nullable_value) necessary? It retains the previous value
          //       beforing setting group.nullable if n = 0.
          let old_nullable_value: bool = group.nullable;

          if n == 0 {
            group.nullable = true;
          }

          if n > m {
            RegexError::InvalidRepeat(self.idx).emit();
          }

          self.idx += 1;

          if self.c() == '?' {
            group.increment_lazy_index();

            self.lazy_set.insert(group.lazy_index);

            if group.nullable {
              group.lazify_own_set(TargetSet::First);
            }
            /* CHECKED algorithmic options: 8/1 else
               {
               self.lazy(group.first_positions, firstpos1);
               group.first_positions.extend(firstpos1.iter());
               pfirstpos = &firstpos1;
               }
            */
            self.idx += 1;
          } else {
            // CHECKED algorithmic options 7/30 if !group.nullable {
            // CHECKED algorithmic options 7/30   self.lazy_set.clear();
            if n < m && self.lazy_set.is_empty() {
              group.greedify_own_set(TargetSet::First);
            }
          }

          // CHECKED added pfirstpos to point to updated group.first_positions with lazy quants
          if !group.nullable {
            // CHECKED algorithmic options 8/1 added to make ((a|b)*?b){2} work
            group.lazify_own_set(TargetSet::First);
          }

          if group.nullable && unlimited {  // {0,} == *
            for p in group.last_positions.iter() {
              self.follow_positions_map
                  .get_mut(p.idx().into())
                  .extend(group.first_positions.iter());
            }
          }
          else if m > 0 {
            if group.iteration * m > MAX_ITER {
              RegexError::ExceedsLimits(self.idx).emit();
            }
            { // scope of more_follow_positions
              // update self.follow_positions_map by virtually repeating sub-regex m-1 times
              let mut more_follow_positions: FollowMap = FollowMap::default();
              for (position, positions_set) in self.follow_positions_map.iter() {
                if position.idx() >= original_position.idx() {
                  for i in 0..m - 1 {
                    for p in positions_set.iter() {
                      more_follow_positions[position.increment_iter(group.iteration * (i + 1))]
                      .insert(p.increment_iter(group.iteration * (i + 1)));
                    }
                  }
                }
              }

              for (position, positions_set) in more_follow_positions.iter() {
                self.follow_positions_map.get_mut(*position).extend(positions_set.iter());
              }
            }

            // add m-1 times virtual concatenation (by indexed positions k.i)
            for i in 0..m - 1 {
              for k in group.last_positions.iter() {
                for j in group.first_positions.iter() {
                  self.follow_positions_map.get_mut(
                    Position(k.idx().into()).increment_iter(group.iteration * i)
                  ).insert(j.increment_iter(group.iteration *( i + 1)));
                }
              }
            }
            if unlimited {
              for k in group.last_positions.iter() {
                for j in group.first_positions.iter() {
                  self.follow_positions_map.get_mut(
                    Position(k.idx().into()).increment_iter(group.iteration * (m - 1))
                  ).insert(j.increment_iter(group.iteration * (m - 1)));
                }
              }
            }
            if old_nullable_value {
              // extend group.first_positions when sub-regex is group.nullable
              let mut more_first_positions = PositionSet::new();
              for i in 1..m {
                for k in group.first_positions.iter() {
                  more_first_positions.insert(k.increment_iter(group.iteration * i));
                }
              }
              group.first_positions.extend(more_first_positions.iter());
            }
            { // scope of new_last_positions
              // n to m-1 are optional with all 0 to m-1 are optional when group.nullable
              let mut new_last_positions = PositionSet::new();
              let start_position = if group.nullable {
                0
              } else {
                n - 1
              };
              for i in start_position..m {
                for k in group.last_positions.iter() {
                  new_last_positions.insert(k.increment_iter(group.iteration * i));
                }
              }
              group.last_positions = new_last_positions;
            }
            group.iteration *= m;
          }
          else { // zero range {0}
            group.first_positions.clear();
            group.last_positions.clear();
            group.lazy_set.clear();
          }
        }
        else {
          RegexError::InvalidRepeat(self.idx).emit();
        }
      }
      else {
        break;
      }
      c = self.c();
    }
    println!("END parse_iterated()");
  }



  fn parse_digit(&mut self) -> usize {
    let mut c: Char;
    let mut k: usize = 0;

    // todo: ...why 7?
    for _i in 0..7 {
      c = self.ci();
      if !c.is_digit() {
        break;
      }

      k = 10 * k + (u8::from(c) - b'0') as usize;
    }
    k
  }


  /**
  ## Stage 4
  Parses exprs of the form `(?^#"[=...)`.
  */
  #[allow(unused_variables)]
  fn parse_sequence(&mut self, group: &mut Group) {
    println!("BEGIN parse_sequence({}) <parse4>", self.idx);

    // todo: necessary?
    group.first_positions.clear();
    group.last_positions.clear();
    group.lazy_set.clear();
    group.nullable = true;
    group.iteration = 1;

    let mut c: Char = self.c();

    if c == '(' {
      self.idx += 1;

      if self.c() == '?' {

        c = self.cr();

        if c == '#' { // (?# comment
          // Fast forward to the end of the comment.
          if let Some(offset) = self.regex[self.idx as usize..].iter().position(|&x| x == b')') {
            self.idx += offset as Index32;
          } else {
            RegexError::MismatchedParens(self.idx).emit();
          }
        } else if c == '^' { // (?^ negative pattern to be ignored (mode: new) {
          self.idx += 1;

          self.parse_alternations(group);

          for p in group.last_positions.iter() {
            self.follow_positions_map.get_mut(p.idx().into()).insert(Position(0).set_accept(true));
          }
        } else if c == '=' { // (?= lookahead
          let lookahead_start: Position = (self.idx - 2).into(); // lookahead at (
          self.idx += 1;

          self.parse_alternations(group);

          group.first_positions.insert(lookahead_start);

          if group.nullable {
            group.last_positions.insert(lookahead_start);
          }

          // do not permit nested lookaheads
          // RJ: Isn't this just an insert?
          //  A: Only if overlapping can only happen when one is a subset of the other.
          { // Scope of `lookahead`
            let lookahead = self.lookahead_map.get_mut(self.idx);
            if lookahead.clone().intersect(lookahead_start.idx()..self.idx).is_empty() {
              lookahead.insert(lookahead_start.idx()..self.idx); // lookstop at )
            }
          }

          for p in group.last_positions.iter() {
            self.follow_positions_map
                .get_mut(p.idx().into())
                .insert(Position(self.idx.into()).set_ticked(true));
          }
          group.last_positions.insert(Position(self.idx.into()).set_ticked(true));
          if group.nullable {
            group.first_positions.insert(Position(self.idx.into()).set_ticked(true));
            group.last_positions.insert(lookahead_start);
          }
        } else if c == ':' {
          self.idx += 1;
          self.parse_alternations(group);
        } else {
          let mut modifier_start: Index32 = self.idx;

          // Store original x/q options, as a recursive call could change them.
          let opt_q: bool = self.options.quote_with_x;
          let opt_x: bool = self.options.x_freespacing;
          let mut active: bool = true;

          loop {
            if c == '-' {
              active = false;
            } else if c == 'q' {
              self.options.quote_with_x = active;
            } else if c == 'x' {
              self.options.x_freespacing = active;
            } else if c != 'i' && c != 'm' && c != 's' {
              RegexError::InvalidModifier(self.idx).emit();
            }

            c = self.cr();

            if !(c != '\0' && c != ':' && c != ')') {
              break;
            }
          }

          if c != '\0' {
            self.idx += 1;
          }

          // enforce (?imqsx) modes
          self.parse_alternations(group);

          active = true;
          loop {
            c = self.at(modifier_start);
            modifier_start += 1;
            if c == '-' {
              active = false;
            } else if c != '\0' && c != 'q' && c != 'x' && c != ':' && c != ')' {
              if active {
                self.modifiers.set(
                  Mode::from(c),
                  modifier_start..self.idx,
                );
              } else {
                self.modifiers.set(
                  Mode::from(c.to_uppercase()),
                  modifier_start..self.idx,
                );
              }
            }
            if c == '\0' || c == ':' || c == ')' {
              break;
            }
          }

          // Restore original x/q option values
          // todo: self.options should only be for global-only modes. All other modes set at init.
          self.options.quote_with_x = opt_q;
          self.options.x_freespacing = opt_x;
        }
      } // end if '?'
      else {
        self.parse_alternations(group);
      }

      // todo: Yeah, this doesn't look right.
      if c != ')' {
        if self.c() == ')' {
          self.idx += 1;
        } else {
          RegexError::MismatchedParens(self.idx).emit();
        }
      }
    }
    else if c == '[' {
      group.first_positions.insert(self.idx.into());
      group.last_positions.insert(self.idx.into());
      group.nullable = false;

      c = self.cr();

      if c == '^' {
        c = self.cr();
      }

      while c != '\0' {
        if c == '[' && self.at(self.idx + 1) == ':' {
          // nested brackets:  `[: ... :]` character class.

          let maybe_closing_index = self.find_at(self.idx + 2, ':');
          if let Some(closing_loc) = maybe_closing_index {
            if self.at(closing_loc + 1) == ']' {
              self.idx = closing_loc + 1;
            }
          }
        } else if c == self.options.escape_character && !self.options.bracket_escapes {
          self.idx += 1;
        }

        c = self.cr();

        if c == ']' {
          break;
        }
      }

      if c == '\0' {
        RegexError::MismatchedBrackets(self.idx).emit();
      }

      self.idx += 1;
    }
    else if (c == '"' && self.options.quote_with_x) || self.escape_at(self.idx) == Some('Q'.into()) {
      let double_quotes: bool = c == '"';

      if !double_quotes {
        self.idx += 1;
      }

      c = self.cr();
      let quote_start_loc: Index32 = self.idx;

      /*
      A bit convoluted, the following just checks for an closing quote matching the kind used to
      open the quote and that the input is not exhausted.
      */
      let quote_condition = match double_quotes {
        true => (c != '"'),
        false => (c != self.options.escape_character)
      };

      if c != '\0' && (quote_condition || self.at(self.idx + 1) != 'E') {
        // Not the end of the quote
        group.first_positions.insert(self.idx.into());
        let mut p: Position = Position(0);
        loop {
          if double_quotes &&
          (c == self.options.escape_character) &&
          (self.at(self.idx + 1) == '"')
          {
            self.idx += 1;
          }

          if p != Position(position::NPOS) {
            self.follow_positions_map
                .get_mut(p.idx().into())
                .insert(self.idx.into());
          }

          p = self.idx.into();
          self.idx += 1;
          c = self.c();

          if !(
                c != '\0' &&
                (!double_quotes || c != '"') &&
                (double_quotes ||
                c != self.options.escape_character ||
                self.at(self.idx + 1) != 'E'
                )
              )
          {
            break;
          }
        }
        group.last_positions.insert(p);
        group.nullable = false;
        self.modifiers.set(Mode::q, quote_start_loc..self.idx - 1);
      }

      if !double_quotes && self.c() != '\0' {
        self.idx += 1;
      }

      if self.c() != '\0' {
        self.idx += 1;
      } else {
        RegexError::MismatchedQuotation(self.idx).emit();
      }
    }
    else if c == '#' && self.options.x_freespacing {
      // Advance self.idx to "the end."
      self.idx =
        match self.find_at(self.idx, '\n') {
          Some(index) => index + 1,       // End of line
          None => self.regex.len() as Index32 // End of string.
        } as Index32;
    }
    else if c.is_whitespace() && self.options.x_freespacing {
      self.idx += 1;
    } else if c == ')' {
      RegexError::MismatchedParens(self.idx).emit();
      //self.idx += 1;
    }
    else if c == '}' {
      RegexError::MismatchedBraces(self.idx).emit();
      //self.idx += 1;
    }
    else if c != '\0' && c != '|' && c != '?' && c != '*' && c != '+' {
      group.first_positions.insert(Position::from(self.idx));
      group.last_positions.insert(Position::from(self.idx));
      group.nullable = false;
      if c == self.options.escape_character {
        self.parse_esc(None);
      } else {
        self.idx += 1;
      }
    } else if self.is_first_group && c != '\0' { // permits empty regex pattern but not empty subpatterns
      RegexError::EmptyExpression(self.idx).emit()
    }
    println!("END parse_sequence() <parse4>");
  }



  /**
  Parse escape character, inserting it into the value wrapped by maybe_chars. This method
  advances `self.idx`.
  */
  fn parse_esc(&mut self, mut maybe_chars: Option<&mut Chars>) -> Char {
    // Increment past the `'\'`
    let mut c: Char = self.cr();

    if c == '0' {
      // `\0177` 	matches an 8-bit character with octal value `177`.
      // (Use `\177` in lexer specifications instead.)
      // Note: The code below requires exactly three octal digits after the initial zero.
      c = Char(0);

      let mut d: u16 = 0;
      for _i in 0..3{
        if !(c.0 < 32 && d >= b'0' as u16 && d <= b'7' as u16){
          break;
        }
        self.idx += 1;
        d = self.c().0;
        c = Char((c.0 << 3) + d - b'0' as u16);
      }

      self.idx += 1;

    }
    else if c == 'x' || c == 'u' {
      // `\x7f` 	  matches an 8-bit character with hexadecimal value `7f`
      // `\x{7f}` 	matches an 8-bit character with hexadecimal value `7f`
      self.idx += 1;
      c = Char(0);

      let skip_curley = match self.c() == '{'{

        true  => {
          if self.at(self.idx + 3) != '}' {
            RegexError::InvalidEscape(self.idx).emit();
          }
          self.idx += 1;
          1
        }

        false => 0

      };

      // Parse exactly two hex digits, placing the value into `c`.
      if !self.c().is_hexdigit() || !self.at(self.idx+1).is_hexdigit(){
        RegexError::InvalidEscape(self.idx).emit();
      }

      for _i in 0..2 {
        let d: u16 = self.c().0;
        c = Char(
          c.0 << 4 + match d > b'9' as u16 {

            true => (d | 0x20) - (b'a' as u16 - 10),

            false => d - b'0' as u16

          }
        );
        self.idx += 1;
      }

      self.idx += skip_curley;

    }
    else if c == 'c' {
      // `\cX`: control character `X` mod 32
      self.idx += 1;
      c = Char(self.c().0 % 32);
      self.idx += 1;
    }
    else if c == 'e' {
      // Escape character
      c = Char(0x1B); // 0x1b == 27 == <ESC>
      self.idx += 1;
    }
    else if c == 'N' {
      // Complement of newline
      if let Some(chars) = &mut maybe_chars {
        chars.insert_pair(Char(0), Char(9));
        chars.insert_pair(Char(11), Char(255));
      }
      self.idx += 1;
      c = Meta::EndOfLine.into();
    }
    else if (c == 'p' || c == 'P') && self.at(self.idx + 1) == '{' {
      // `\p{alnum}`: Posix character class.
      self.idx += 2;
      if let Some(chars) = &mut maybe_chars {

        **chars |= *self.parse_char_class();
        //c = self.c();

        if c == 'P' {
          chars.flip();
        }
        // Curley must be closed.
        if self.c() == '}' {
          self.idx += 1;
        } else {
          RegexError::InvalidEscape(self.idx).emit();
        }
      }
      else {
        self.idx += 1;
        c = self.c();
        while c != '\0' && c != '}' {
          self.idx += 1;
          c = self.c();
        }
        if c == '}' {
          self.idx += 1;
        } else {
          RegexError::InvalidEscape(self.idx).emit();
        }
      }
      c = Meta::EndOfLine.into();
    }
    else if c != '_' {

      // If it's an escape character, convert to the ASCII character it refers to.
      if let Some(new_c) = Char::try_from_escape(c) {
        c = new_c;
      }
      else {
        if let Some(_) = chars::add_posix_class(c, &maybe_chars) {
          c = Meta::EndOfLine.into();
        }
      }

      self.idx += 1;
    }

    if let Some(chars) = maybe_chars {
      // We signal to not insert `c` by setting it to an arbitrary `Meta` variant.
      if !c.is_meta() {
        chars.insert(c);
      }
    }

    return c;
  }



  /**
  Parses the name of the character class beginning at `self.idx`, returning its associated
  `Chars`. This method advances `self.idx` to one past the end of the name.
  */
  fn parse_char_class(&mut self) -> &Chars {
    for (i, name) in POSIX_CLASS_NAMES.iter().enumerate() {
      if self.regex[self.idx as usize..(self.idx as usize + name.len())]
             .to_ascii_lowercase()  == name.to_ascii_lowercase().as_bytes()
      {
        self.idx += name.len() as Index32;
        println!("posix({})", name);
        return POSIX_CLASSES[i];
      }
    }
    // Not the name of a character class name.
    RegexError::InvalidClass(self.idx).emit();
  }


  fn debug_log_position_set(&self, positions: &PositionSet, indent_level: usize) {
    //println!("{} = {{", target_set);
    print!("{}{}", " ".repeat(indent_level*2),
      positions.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ")
    );
  }


  fn debug_log_follow_map(&self, indent_level: usize) {

    for (position, positions_set) in self.follow_positions_map.iter() {
      print!("{}{}{} ) = {{", " ".repeat(indent_level*2), "follow_positions_map(", position);

      self.debug_log_position_set(positions_set, indent_level);
      println!(" }}");
    }



  }



}


/// Makes everything in positions greedy.
// todo: make this a method on `Positions`. Requires makes `Positions` a struct.
pub fn greedify(positions: &PositionSet) -> PositionSet{

  let mut new_positions: PositionSet = PositionSet::new();
  for p in positions.iter() {
    let new_position =
    match p.lazy() != 0 {
      true  => *p,
      false => p.set_greedy(true)
    };
    new_positions.insert(new_position);
  }
  /*
    CHECKED algorithmic options: 7/29 guard added: p.lazy() ? *p : p.greedy(true)
    CHECKED 10/21 pos1.insert(p.lazy(0).greedy(true));
    pos.swap(pos1);
  */
  new_positions
}










#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn parse_modifiers() {
    let parser = Parser::new("(?imsux)abc*|ghj", "");
  }

  #[test]
  fn parse_options(){
    let parser = Parser::new("", "bimopf=one.h, one.cpp, two.cpp, stdout;qrswx");
  }

}
