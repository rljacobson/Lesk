#![allow(dead_code)]

/*!

Parses and compiles a regex. Parsing and compilation are not altogether separate. Parsing of
character lists `[abc]` is deferred until compilation, as there is almost nothing to parse.

*/


use std::cell::RefMut;
use std::cmp::max;
use std::collections::hash_map::Entry;
use std::ops::Deref;
use std::time::Duration;

use defaultmap::DefaultHashMap;
use quanta::Clock;

use chars;
use chars::{ALL_CHARS, NON_NEWLINE_CHARS, POSIX_CLASS_NAMES, POSIX_CLASSES};
use error::RegexError;
use group::{Group, TargetSet};
use limits::{MAX_INDEX, MAX_ITER};
use modifier::{Mode, Modifiers};
use options::Options;
use position::VcPositionSet;
use state::{State, StateNextIterator, VcState};

use crate::relesk::compiler::Compiler;
use crate::valuecell::ValueCell;

use super::*;
use std::collections::BTreeSet;

static END_ESCAPE_OPTION: &[u8; 39] = b"0123456789<>ABDHLNPSUWXbcdehijklpsuwxz\0";
static META_OPS: &[u8; 9] = b".^$([{?*+";

#[derive(Default)]
pub struct Parser<'a> {
  options: Options,
  //< Pattern compiler options
  modifiers: Modifiers,
  //< Describes which modifiers are active at which positions
  // in the regex
  regex: &'a [u8],  //< Regular expression string as bytes

  idx: Index32,
  //< Cursor into `self.regex`
  group: ValueCell<Group>,
  //< The outermost matching group representing the entire regex
  next_group_idx: Index32,
  //< A "global" variable keeping track of the index for the next new group
  is_first_group: bool,             //< Only true while parsing the outer-most group



  // Shared?
  pub lazy_set: HashSet<Lazy8>, //< Group indices in the regex that are lazily matched.

  /// For each position in the group, which positions can follow it.
  pub follow_positions_map: FollowMap,
  pub start_positions: PositionSet, //< Accumulates first positions


  /**

  Maps a top-level group index to the set of lookaheads for that index. The keys are `Index32`s, and
  the values are `PositionSet`s. The `lookahead` `PositionSet` for a subgroup is obtained by using
  `group.idx` as a key.

  From the docs:

    A lookahead pattern φ(?=ψ) matches φ only when followed by pattern ψ. The text matched by ψ is
    not consumed.

    Boost.Regex and PCRE2 matchers support lookahead φ(?=ψ) and lookbehind φ(?<=ψ) patterns that may
    appear anywhere in a regex. The RE/flex matcher supports lookahead at the end of a pattern,
    similar to Trailing context.

  Subgroups are Consulted in `parse_iterated` (parse2) and filled in `parse_alternations` (parse4).
  */
  pub lookahead_map: DefaultHashMap<Index32, IndexRanges>,

  // Compiler

  start: VcState,
  moves: MoveVec,
  // todo: only needed if reading in before parse??
  prefix: Vec<u8>, //< pattern prefix, shorter or equal to 255 bytes

  vertex_count: usize,
  //< number of finite state machine vertices |V|
  edge_count: usize,       //< number of finite state machine edges |E|

  subpattern_endpoints: Vec<Index32>,
  //< entries point to the subpattern's ending '|' or '\0'
  subpattern_is_accepting: Vec<bool>,  //< true if subpattern n is accepting (state is reachable)




  // Benchmark/Diagnostic data.
  pub parse_time: Duration,
  pub vertices_time: Duration,
  //< ms elapsed DFA vertices construction time
  pub edges_time: Duration,
  //< ms elapsed DFA edges construction time
  pub code_words_time: Duration, //< ms elapsed code words assembly time
}

impl<'a> Parser<'a> {
  pub fn new<'p>(regex: &'p str, options_string: &'p str) -> Parser<'p> {
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
    self.start = VcState::new(State::default());
    // Compile the NFA
    self.compile();


    // Assemble DFA opcode tables or direct code
    let mut compiler = Compiler::new(
      self.regex,
      self.start.clone(),
      self.prefix.clone(),
      &self.options
    );
    // Compilers DFA from NFA
    compiler.assemble();
  }

  // region Inlined Methods

  #[must_use]
  fn follow_positions<I>(&mut self, index: I, _source_file: &str, _source_line: u32)
    -> RefMut<PositionSet>
    where I: Into<Position>
  {
    let p: Position = index.into();
    //println!("{}:{}: follow_positions_map[{}]", source_file, source_line, &p);
    self.follow_positions_map.entry(p).or_default().borrow_mut()
  }

  /// Returns the character at the index `idx` of the regular expression.
  #[must_use]
  pub(crate) fn at(&self, idx: Index32) -> Char {
    if idx >= self.regex.len() as Index32 {
      // We do not return `Option<Char>` to keep unwrapping to reasonable levels.
      return '\0'.into();
    }
    //println!("self.at({}) == {}", idx, self.regex[idx as usize] as char);
    Char::from(self.regex[idx as usize])
  }

  /// Same as `at()` but assumes `idx=self.idx`.
  #[must_use]
  fn c(&self) -> Char {
    self.at(self.idx)
  }

  /// Same as `c()` but post-increments `self.idx`.
  #[must_use]
  fn ci(&mut self) -> Char {
    self.idx += 1;
    self.at(self.idx - 1)
  }


  /// Same as `c()` but PRE-increments `self.idx`.
  #[must_use]
  fn cr(&mut self) -> Char {
    self.idx += 1;
    self.at(self.idx)
  }

  fn next_group_index(&mut self) -> Index32 {
    self.next_group_idx =
    self.next_group_idx.checked_add(1).unwrap_or_else(
      || {
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
  pub fn escape_at(&self, loc: Index32) -> char {
    if self.at(loc) == self.options.escape_character {
      return char::from(self.at(loc + 1));
    }
    '\0'
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
    .position(|&x| x == (c as u8))
    .and_then(|index| Some(idx + (index as Index32)))
  }


  // Compiler-related inlined methods


  /**
  Check if subpattern is reachable by a match.

  @returns true if subpattern is reachable
  */
  #[must_use]
  pub fn is_reachable(&self, choice: GroupIndex32) -> bool {
    return choice >= 1 &&
    choice <= self.subpattern_endpoints.len() as GroupIndex32 &&
    self.subpattern_is_accepting[choice as usize - 1];
  }


  // endregion

  // region Parser Methods

  /**
  ## Stage 0A
  The top-level `parse` function parses modifiers of the form `(?imqsx-imqsx)`, alternations, and
  string literal patterns and then calls `parse_anchors` to parse its subpatterns.

  Only called once, and calls `parse_anchors`. Recursive calls are to `parse_alternations`.

  ```

  parse ⟶ parse_anchors ⟶ parse_iterated ⟶ parse_grouped
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
      self.group.borrow_mut().idx = self.next_group_index();
      self.group.borrow_mut().is_first_group = true;

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
                if c == '\0' || c == self.options.escape_character && self.at(end + 1) == 'E' {
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
          let mut c: Char = self.ci();
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
              c = self.ci();
              // If it's an escape character, convert to the ASCII character it refers to.
              if let Some(descaped_c) = Char::try_from_escape(c) {
                c = descaped_c
              }
            }
          } else if c >= 'A' && c <= 'Z' && self.options.insensitive_case {
            c = c.to_lowercase();
          }
          string_literal.push(c.into());
        }
        let next_idx = self.next_group_index();
        self.group.borrow_mut().insert_string(&string_literal, next_idx);
      } else {
        let group = self.group.clone();
        let mut group_ref = group.borrow_mut();
        group_ref.is_first_group = true;
        group_ref.lazy_set = LazySet::new();

        self.parse_anchors(&mut group_ref);

        group_ref.subpattern_endpoints.push(self.idx);

        self.start_positions.append(&mut group_ref.first_positions);
        if group_ref.nullable {
          group_ref.append_idx_as_lazy_accepted(&mut self.start_positions);
        }
        group_ref.append_idx_for_last_positions(&mut self.follow_positions_map);
      }

      if self.ci() != '|' {
        break;
      }
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

    self.subpattern_endpoints = self.group.borrow().subpattern_endpoints.clone();

    #[cfg(feature = "DEBUG")]
    {
      print!("start_positions = {{");
      debug_log_position_set(&self.start_positions, 0);
      println!("}}");
      self.debug_log_follow_map(0);

      println!("Strings = {{ {} }}",
               self.group.borrow().string_trie.iter().map(
                 |x| std::str::from_utf8(x.0.as_slice()).unwrap().to_string()
               ).collect::<Vec<String>>().join(", ")
      );
    }

    println!("END parse()");
    println!("Parse time: {}μs", self.parse_time.as_micros());
  }


  /**
  ## Stage 0B
  Parse "multiple modifiers mode," e.g. `(?imsux-imsux:φ)`, where the modifiers before the dash are
  enabled and the mode modifiers after the dash are disabled.
  */
  fn parse_global_modifiers(&mut self) {
    //println!("BEGIN parse_global_modifiers() <parse0B>");

    if self.c() == '(' && self.at(1) == '?' {
      self.idx = 2;

      // Compute the ending location of the option expression.
      while self.c().is_alphanumeric() || self.c() == '-' {
        self.idx += 1;
      }

      if self.c() == ')' {
        let mut active: bool = true;
        self.idx = 2;

        let mut c: char = self.c().into();
        while c != ')' {
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
          c = self.cr().into();
        }
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
  fn parse_alternations(&mut self, group: &mut Group) {
    //println!("BEGIN parse_alternations({})", self.idx);

    // Called with provided group
    self.parse_anchors(group);

    let mut new_group = Group::default();
    new_group.idx = group.idx;
    new_group.lazy_index = group.lazy_index;

    while self.c() == '|' {
      self.idx += 1;

      self.parse_anchors(&mut new_group);
      // Update the old values.
      group.first_positions.extend(new_group.first_positions.iter());
      group.last_positions.extend(new_group.last_positions.iter());
      group.lazy_set.extend(new_group.lazy_set.iter());


      group.nullable = new_group.nullable || group.nullable;
      group.iteration = max(new_group.iteration, group.iteration);
    }


    //group.debug_log_position_set(TargetSet::First, 0);
    //group.debug_log_position_set(TargetSet::Last, 0);
    //println!("END parse_alternations");
  }


  /**
  ## Stage 2
  Parses anchored groups
  */
  fn parse_anchors(&mut self, group: &mut Group) {
    //println!("BEGIN parse_anchors({}) <parse2>", self.idx);

    let mut anchor_positions: PositionSet = PositionSet::default();
    if group.is_first_group {
      loop {
        if self.options.x_freespacing {
          while self.c().is_whitespace() { self.idx += 1; }
        }

        // Check for BOL anchor
        if self.c() == '^' {
          anchor_positions.insert(Position(self.ci().0 as u64));
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
        else if self.escapes_at(self.idx, b"ij").is_some() {
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
    new_group.idx = group.idx;
    new_group.lazy_index = group.lazy_index;
    new_group.is_first_group = false;

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

      if !group.lazy_set.is_empty() {
        /*
          CHECKED self is an extra rule for + only and (may) not be needed for *
          CHECKED algorithmic options: self.lazy(firstpos1); does not work for (a|b)*?a*b+,
          below works
        */

        //Positions firstpos2;
        //lazy(lazyset, firstpos1, firstpos2);
        //set_insert(firstpos1, firstpos2);
        let mut lazy_first_positions = group.lazify(&new_group.first_positions);
        new_group.first_positions.append(&mut lazy_first_positions);

        /*
          if (lazyset1.is_empty())
          greedy(firstpos1); // CHECKED algorithmic options: 8/1 works except fails for
            ((a|b)*?b){2} and (a|b)??(a|b)??aa
        */
      }

      if group.nullable {
        //group.extend_with(TargetSet::First, &new_group.first_positions);
        group.first_positions.extend(new_group.first_positions.iter());
      }

      // The first positions of this one are the last positions of the previous
      for p in group.last_positions.iter() {
        self.follow_positions(p.index_with_iter(), file!(), line!())
            .extend(new_group.first_positions.iter());
        //debug_display_appending(&new_group.first_positions);
      }

      if new_group.nullable {
        group.last_positions.extend(new_group.last_positions.iter());
        group.lazy_set.extend(new_group.lazy_set.iter()); // CHECKED 10/21
      } else {
        std::mem::swap(&mut group.last_positions, &mut new_group.last_positions);
        std::mem::swap(&mut group.lazy_set, &mut new_group.lazy_set);
        group.nullable = false;
      }

      // CHECKED 10/21 set_insert(group.lazy_set, lazyset1);
      group.iteration = max(new_group.iteration, group.iteration);
      c = self.c();
    }


    for p in anchor_positions.iter() {
      for k in group.last_positions.iter() {
        if self.at(k.idx()) == ')'
            // todo: Can group.idx be trusted to give the right `lookahead` set?
            && self.lookahead_map[group.idx].contains(&k.idx())
        {
          self.follow_positions(p.index_with_iter(), file!(), line!()).insert(*k);

        }

        self.follow_positions(k.index_with_iter(), file!(), line!())
            .insert(
              p.set_anchor(!group.nullable || k.index_with_iter() != p.index_with_iter())
            );
      }

      group.last_positions.clear();
      group.last_positions.insert(*p);

      if group.nullable {
        group.first_positions.insert(*p);
        group.nullable = false;
      }
    }

    /*
    group.debug_log_position_set(TargetSet::First, 0);
    group.debug_log_position_set(TargetSet::Last, 0);
    self.debug_log_follow_map(0);

    println!("END parse_anchors()");
    */
  }


  /**
  ## Stage 3
  Parses repeated/optional subexpressions: `*`, `+`, `?`
  */
  fn parse_iterated(&mut self, group: &mut Group) {
    //println!("BEGIN parse_iterated({}) <parse3>", self.idx);

    let original_position: Position = Position(self.idx.into());

    // Called with original global values
    self.parse_grouped(group);
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
        c = self.cr();
      }
    }

    loop {
      if c == '*' || c == '+' || c == '?' {
        if c == '*' || c == '?' {
          group.nullable = true;
        }
        if self.cr() == '?' {
          group.increment_lazy_index();
          group.lazy_set.insert(group.lazy_index);

          if group.nullable {
            group.lazify_own_set(TargetSet::First);
          }
          self.idx += 1;
        } else {
          // CHECKED algorithmic options: 7/30 if !group.nullable {
          // CHECKED algorithmic options: 7/30   group.lazy_set.clear();
          group.first_positions = greedify(&group.first_positions);
        }
        if c == '+' && !group.nullable && !group.lazy_set.is_empty() {
          let more_first_positions: PositionSet = group.lazify(&group.first_positions);
          for p in group.last_positions.iter() {
            self.follow_positions(p.index_with_iter(), file!(), line!())
                .extend(more_first_positions.iter());
          }
          //debug_display_appending(&more_first_positions);
          group.first_positions.extend(more_first_positions.iter());
        } else if c == '*' || c == '+' {
          //println!("Extending follow positions due to * operator.");
          for p in group.last_positions.iter() {
            //print!("self.follow_positions_map[{}] += ", Position::from(p.idx
            //()));

            //group.debug_log_position_set(TargetSet::First, 0);
            self.follow_positions(p.index_with_iter(), file!(), line!())
                .extend(group.first_positions.iter());
            //debug_display_appending(&group.first_positions);
          }
        }
      } else if c == '{' {
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
          let old_nullable_value: bool = group.nullable;

          if n == 0 {
            group.nullable = true;
          }

          if n > m {
            RegexError::InvalidRepeat(self.idx).emit();
          }


          if self.cr() == '?' {
            group.increment_lazy_index();

            group.lazy_set.insert(group.lazy_index);

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
            // CHECKED algorithmic options 7/30   group.lazy_set.clear();
            if n < m && group.lazy_set.is_empty() {
              group.greedify_own_set(TargetSet::First);
            }
          }

          //if !group.nullable {
          //  group.lazify_own_set(TargetSet::First);
          //}

          // CHECKED added pfirstpos to point to updated group.first_positions with lazy quants
          // We need `lazy_first_positions` to potentially hold the value produced inside the first
          // if block below. Otherwise it is unused.
          #[allow(unused_assignments)]
          let mut lazy_first_positions: PositionSet = PositionSet::new();
          let mut first_position_ptr: &PositionSet = &group.first_positions;
          // CHECKED algorithmic options 8/1 added to make ((a|b)*?b){2} work
          if !group.nullable && !group.lazy_set.is_empty() {
            lazy_first_positions = group.lazify(&group.first_positions);
            first_position_ptr = &lazy_first_positions;
          }

          if group.nullable && unlimited {  // {0,} == *
            for p in group.last_positions.iter() {
              self.follow_positions(p.index_with_iter(), file!(), line!())
                  .extend(first_position_ptr.iter());
              //debug_display_appending(first_position_ptr);
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
                    for p in positions_set.borrow().iter() {
                      more_follow_positions
                        .entry(position.increment_iter(group.iteration * (i + 1)))
                        .or_default()
                        .borrow_mut()
                        .insert(p.increment_iter(group.iteration * (i + 1)));

                    }
                  }
                }
              }

              for (position, positions_set) in more_follow_positions.iter() {
                self.follow_positions(*position, file!(), line!()).extend(positions_set.borrow().iter());
                //debug_display_appending(&*positions_set.borrow());
              }
            }

            // add m-1 times virtual concatenation (by indexed positions k.i)
            for i in 0..m - 1 {
              for k in group.last_positions.iter() {
                for j in first_position_ptr.iter() {
                  self.follow_positions(
                    Position(k.index_with_iter().into()).increment_iter(group.iteration * i), file!(),
                    line!()
                  ).insert(j.increment_iter(group.iteration * (i + 1)));
                  //println!("Inserting the item {}", j.increment_iter(group.iteration * (i + 1)));
                }
              }
            }
            if unlimited {
              for k in group.last_positions.iter() {
                for j in first_position_ptr.iter() {
                  self.follow_positions(
                    Position(
                      k.index_with_iter().into())
                       .increment_iter(group.iteration * (m - 1)), file!(), line!()
                  ).insert(j.increment_iter(group.iteration * (m - 1)));
                }
              }
            }
            if old_nullable_value {
              // If subgroup was already nullable, take all first_positions and add them again with
              // all possible values for iteration set (extend group.first_positions when sub-regex
              // is group.nullable).
              let mut more_first_positions = PositionSet::new();
              for i in 1..m {
                for k in first_position_ptr.iter() {
                  more_first_positions.insert(k.increment_iter(group.iteration * i));
                }
              }
              group.first_positions.append(&mut more_first_positions);
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
          } else { // zero range {0}
            group.first_positions.clear();
            group.last_positions.clear();
            group.lazy_set.clear();
          }
        } else {
          RegexError::InvalidRepeat(self.idx).emit();
        }
      } else {
        break;
      }
      c = self.c();
    }

    /*
    //println!("group.first_positions = ");
    group.debug_log_position_set(TargetSet::First, 0);
    //println!("group.last_positions = ");
    group.debug_log_position_set(TargetSet::Last, 0);
    self.debug_log_follow_map(0);

    println!("END parse_iterated()");
    */
  }


  fn parse_digit(&mut self) -> usize {
    let mut c: Char;
    let mut k: usize = 0;

    for _i in 0..7 {
      c = self.cr();
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
  fn parse_grouped(&mut self, group: &mut Group) {
    //println!("BEGIN parse_grouped({}) <parse4>", self.idx);

    // todo: Should this just be a new group?
    group.first_positions.clear();
    group.last_positions.clear();
    group.lazy_set.clear();
    group.nullable = true;
    group.iteration = 1;

    let mut c: Char = self.c();

    if c == '(' {
      if self.cr() == '?' {
        c = self.cr();

        if c == '#' { // (?# comment
          // Fast forward to the end of the comment.
          if let Some(offset) = self.regex[self.idx as usize..].iter().position(|&x| x == b')') {
            self.idx += offset as Index32 + 1;
          } else {
            RegexError::MismatchedParens(self.idx).emit();
          }
        }
        else if c == '^' { // (?^ negative pattern to be ignored (mode: new) {
          self.idx += 1;

          self.parse_alternations(group);

          for p in group.last_positions.iter() {
            self.follow_positions(p.index_with_iter(), file!(), line!())
                .insert(Position(0).set_accept(true));
          }
        }
        else if c == '=' { // (?= lookahead
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

          let ticked_position = Position(self.idx.into()).set_ticked(true);
          for p in group.last_positions.iter() {
            self.follow_positions(p.index_with_iter(), file!(), line!())
                .insert(ticked_position);
          }
          group.last_positions.insert(ticked_position);
          if group.nullable {
            group.first_positions.insert(ticked_position);
            group.last_positions.insert(lookahead_start);
          }
        }
        else if c == ':' {
          self.idx += 1;
          self.parse_alternations(group);
        }
        else {
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

            if c == '\0' || c == ':' || c == ')' {
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
          self.options.quote_with_x = opt_q;
          self.options.x_freespacing = opt_x;
        }
      } // end if '?'
      else {
        self.parse_alternations(group);
      }

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
    else if (c == '"' && self.options.quote_with_x) || self.escape_at(self.idx) == 'Q' {
      let double_quotes: bool = c == '"';

      if !double_quotes {
        self.idx += 1;
      }

      let quote_start_loc: Index32 = self.cr().0 as Index32;
      c = self.c();

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
            let idx = self.idx;
            println!("INSERTING {} INTO FOLLOW.", idx);
            self.follow_positions(p.index_with_iter(), file!(), line!()).insert(idx.into());
          }

          p = Position(self.ci().0 as u64);
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
    }
    else if c == ')' {
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
    }
    else if self.is_first_group && c != '\0' { // permits empty regex pattern but not empty subpatterns
      RegexError::EmptyExpression(self.idx).emit()
    }

    /*
    group.debug_log_position_set(TargetSet::First, 0);
    group.debug_log_position_set(TargetSet::Last, 0);
    self.debug_log_follow_map(0);
    println!("END parse_grouped() <parse4>");
    */
  }


  /**
  Parse escape character, inserting it into the value wrapped by maybe_chars. This method
  advances `self.idx`.
  */
  fn parse_esc(&mut self, mut maybe_chars: Option<&mut Chars>) -> Char {
    // Increment past the `'\'`
    let mut c: Char = self.cr();

    match char::from(c) {
      '0' => {
        // `\0177` 	matches an 8-bit character with octal value `177`.
        // (Use `\177` in lexer specifications instead.)
        c = Char(0);

        let mut d: Char = self.cr();
        for _i in 0..3 {
          if !(c.0 < 32 && d >= '0' && d <= '7') {
            break;
          }
          c = Char((c.0 << 3) + d.0 - b'0' as u16);
          d = self.cr();
        }
      }

      | 'x'
      | 'u' => {
        // `\x7f` 	  matches an 8-bit character with hexadecimal value `7f`
        // `\x{7f}` 	matches an 8-bit character with hexadecimal value `7f`
        self.idx += 1;
        c = Char(0);

        let skip_curley = match self.c() == '{' {
          true => {
            if self.at(self.idx + 3) != '}' {
              RegexError::InvalidEscape(self.idx).emit();
            }
            self.idx += 1;
            1
          }

          false => 0
        };
        /*
        // Parse two hex digits, placing the value into `c`.
        if !self.c().is_hexdigit() || !self.at(self.idx + 1).is_hexdigit() {
          RegexError::InvalidEscape(self.idx).emit();
        }
        */
        for _i in 0..2 {
          if self.c().is_hexdigit() { break; }
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

      'c' => {
        // `\cX`: control character `X` mod 32
        c = Char(self.cr().0 % 32);
        self.idx += 1;
      }

      'e' => {
        // Escape character
        c = Char(0x1B); // 0x1b == 27 == <ESC>
        self.idx += 1;
      }

      'N' => {
        // Complement of newline
        if let Some(chars) = &mut maybe_chars {
          chars.insert_pair(Char(0), Char(9));
          chars.insert_pair(Char(11), Char(255));
        }
        self.idx += 1;
        c = Meta::EndOfLine.into();
      }

      | 'p'
      | 'P' if self.at(self.idx + 1) == '{' => {
        // `\p{alnum}`: Posix character class.
        self.idx += 2;
        if let Some(chars) = &mut maybe_chars {
          **chars |= *self.parse_char_class();
          //c = self.c();

          if c == 'P' {
            chars.flip();
          }
          // Curly must be closed.
          if self.c() == '}' {
            self.idx += 1;
          } else {
            RegexError::InvalidEscape(self.idx).emit();
          }
        } else {
          c = self.cr();
          while c != '\0' && c != '}' {
            c = self.cr();
          }
          if c == '}' {
            self.idx += 1;
          } else {
            RegexError::InvalidEscape(self.idx).emit();
          }
        }
        c = Meta::EndOfLine.into();
      }

      _t if _t != '_' => {
        // If it's an escape character, convert to the ASCII character it refers to.
        if let Some(new_c) = Char::try_from_escape(c) {
          c = new_c;
        } else {
          if let Some(_) = chars::add_posix_class(c, &maybe_chars) {
            c = Meta::EndOfLine.into();
          }
        }
        self.idx += 1;
      }

      _ => {
        // Must be that `c == '_'`
      }
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
      .to_ascii_lowercase() == name.to_ascii_lowercase().as_bytes()
      {
        self.idx += name.len() as Index32;
        //println!("posix({})", name);
        return POSIX_CLASSES[i];
      }
    }
    // Not the name of a character class name.
    RegexError::InvalidClass(self.idx).emit();
  }


  // endregion

  // region Compiler Methods


  fn compile(&mut self) {
    println!("BEGIN compile()");

    // init stats and timers
    self.vertex_count = 0;
    self.edge_count = 0;
    self.edges_time = Duration::default();

    // Timing
    let timer = Clock::new();
    let vertex_start_time = timer.start();
    let mut edge_start_time;

    // Construct the DFA
    self.subpattern_is_accepting.resize(self.subpattern_endpoints.len(), false);

    // todo: Isn't start guaranteed to be empty at this point?
    self.start.deref().borrow_mut().trim_lazy();

    /*
    See https://swtch.com/~rsc/regexp/regexp1.html.
    The table takes a list of states and produces the DFA state to which it corresponds or
    creates a new DFA state associated to the list if needed.
    */
    let mut table: HashMap<VcPositionSet, VcState> = HashMap::new();

    { // Scope of `start_ref`
      let start_rc = self.start.clone();
      let start_ref = start_rc.borrow_mut();

      // To setup for compilation, `self.start_positions` becomes the `PositionSet` for `self.start`.
      //let start_positions: VcPositionSet = self.start.borrow().positions.clone();
      std::mem::swap(&mut *start_ref.positions.borrow_mut(), &mut self.start_positions);
      println!("Compiling {} start positions and {} subpattern_endpoints.",
               start_ref.positions.borrow().len(),
               self.subpattern_endpoints.len()
      );

      // Start state should only be discoverable (to possibly cycle back to) if no string tree DFA
      // was constructed
      //if start_ref.tnode.is_none() {
      table.entry(start_ref.positions.clone()).insert(self.start.clone());
      //}
    }

    // Previous added state in the state.next.next.next... chain.
    let mut last_state: VcState = self.start.clone();
    for state in StateNextIterator::new(self.start.clone()) {
      // Set the timer.
      edge_start_time = timer.start();

      // Use the string tree DFA accept state, if present
      /*
      if let Some(root_node) = &state.borrow_mut().tnode {
        // todo: Is this branchless version equivalent to the commented code? I.e. is
        //  state.accept always 0?
        let accept_value = max(root_node.borrow().accept, 0);
        state.borrow_mut().accept = accept_value;
        // if root_node.accept > 0 {
        //   state.accept = root_node.accept;
        // }
      }
      */

      self.compile_transition(state.clone());

      /*
      if let Some(root_node) = &state.borrow().tnode {
        let root_ref = root_node.borrow();
        // merge tree DFA transitions into the final DFA transitions to target states
        if moves.is_empty() {
          // no DFA transitions: the final DFA transitions are the tree DFA transitions to target states
          for c in Chars::from(CharClass::ASCII) &
          Chars::from(root_ref.edge.keys().copied().collect::<Vec<u8>>())
          {
            let new_state =  self.add_edge(state.clone(), c);
            last_state.borrow_mut().next = Some(new_state.clone());
            last_state = new_state;
          }
        }
        else {
          // let mut moves = self.moves.borrow_mut();
          // combine the tree DFA transitions with the regex DFA transition moves
          let mut chars: Chars =
          Chars::from(CharClass::ASCII) &
          Chars::from(root_ref.edge.keys().copied().collect::<Vec<u8>>());
          if self.options.insensitive_case {
            // Also add the uppercase versions of all lower case letters in `root_ref.edge`.
            chars.make_case_insensitive();
          }

          let mut move_indices_to_remove: Vec<usize> = Vec::new();
          for (i, (ref mut move_chars, move_positions))
          in moves.iter_mut().enumerate()
          {
            if chars.intersects(move_chars) {

              // tree DFA transitions intersect with self DFA transition move
              let common: Chars = chars & *move_chars;
              chars -= common;
              *move_chars -= common;
              if move_chars.is_empty() {
                move_indices_to_remove.push(i);
              }

              for c in common{
                let new_state = self.add_edge(state.clone(), c);
                last_state.borrow_mut().next = Some(new_state.clone());
                last_state = new_state.clone();
                new_state.borrow_mut().positions = move_positions.clone();
              }
            }
          }
          // todo: Make this more efficient. See https://stackoverflow.com/a/63294593/4492422.
          //       Edit - actually, `swap_remove` or `retain` are better.
          { // scope of moves
            // let mut moves = self.moves.borrow_mut();
            for i in move_indices_to_remove.iter().rev(){
              moves.remove(*i);
            }
          }
          if self.options.insensitive_case {
            // Normalize by removing upper case if option i (case insensitive matching) is enabled
            chars -= CharClass::Upper.into();
          }

          for c in chars {
            let new_state =  self.add_edge(state.clone(), c);
            last_state.borrow_mut().next = Some(new_state.clone());
            last_state = new_state;
          }

        }
      }
      */

      self.edges_time += timer.delta(edge_start_time, timer.end());

      for (ref mut position, positions_set) in self.moves.iter_mut()
      {
        // (position, positions_set): (Chars, Positions)
        if !ValueCell::borrow(positions_set).is_empty() {
          let entry = table.entry(positions_set.clone());
          let target_state: &mut VcState =
          match entry {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) =>
              entry.insert(State::with_pos(positions_set.clone()))
          };
          last_state.borrow_mut().next = Some(target_state.clone());
          last_state = target_state.clone();
          // RJ: Connect every NFA state in the DFA state to the target_state.
          // todo: What does this do that is different from `Chars::hi()`/`Chars::lo()`?
          let mut lo: Char = position.lo();
          let max: Char = position.hi();
          while lo <= max {
            if position.contains(lo) {
              let mut hi: Char = lo + 1u16;
              while hi <= max && position.contains(hi) {
                hi += 1;
              }
              hi -= 1;
              // Now hi is one less than the smallest Char not contained in position, or max+1 if
              // position contains them all. Note this is not the same as the largest Char in
              // `position` if the chars are not contiguous.

              #[cfg(feature = "REVERSE_ORDER_EDGE_COMPACT")] //if REVERSE_ORDER_EDGE_COMPACT == -1
              { state.borrow_mut().edges.insert(lo, (hi, target_state.clone())); }
              #[cfg(not(feature = "REVERSE_ORDER_EDGE_COMPACT"))]
              { state.borrow_mut().edges.insert(hi, (lo, target_state.clone())); }

              self.edge_count += (hi.0 - lo.0 + 1) as usize;
              lo = hi + 1;
            }
            lo += 1;
          }
        }
      }

      let accept_value = state.borrow_mut().accept;
      if accept_value > 0 && accept_value as usize <= self.subpattern_endpoints.len() {
        self.subpattern_is_accepting[(accept_value - 1) as usize] = true;
      }

      self.moves.clear();
      self.vertex_count += 1;
    }

    self.vertices_time += timer.delta(vertex_start_time, timer.end()) - self.edges_time;
    println!("END compile()");
    println!("\nVertices: {}\nEdges:{}", self.vertex_count, self.edge_count);
    println!("Vertices time: {}μs", self.vertices_time.as_micros());
    println!("Edges time: {}μs\n", self.edges_time.as_micros());
  }


  fn compile_transition(&mut self, state: VcState) {
    println!("BEGIN compile_transition()");

    { // Scope of references below.
      let mut state_ref = state.borrow_mut();
      let positions_rc = state_ref.positions.clone();
      let positions_ref = positions_rc.borrow_mut();
      for k in positions_ref.iter() {
        if k.is_accept() {

          // pick lowest nonzero accept index
          if state_ref.accept == 0 || k.accepts() < state_ref.accept {
            state_ref.accept = k.accepts();
          }

          // RJ Question: But couldn't we have set state.accept = 0 in previous if?
          // Answer: No, because we are in k.is_accept() branch.
          if k.accepts() == 0 {
            // Redo if first position is accepting.
            state_ref.redo = true;
          }
        } else {
          let mut c: Char = self.at(k.idx());

          let literal: bool = self.modifiers.is_set(k.idx(), Mode::q);

          if (c == '(' || c == ')') && !literal {
            let mut n: Lookahead16 = 0;
            if c == '(' {
              println!("LOOKAHEAD HEAD");
            } else {
              println!("LOOKAHEAD TAIL");
            }

            { // Scope of lookahead_map
              //let lookahead_map_rc = self.lookahead_map.clone();
              //let lookahead_map = lookahead_map_rc.borrow_mut();
              for (hops, (position, locations)) in self.lookahead_map.iter().enumerate()
              {
                //if let Some(range) = locations.contains(k.idx()) {
                if locations.contains(&k.idx()) {
                  println!("{} {} ({}) {}", state.borrow().accept, position, true, n);

                  // todo: What is the distance from locations.begin() to range? Is it the index of
                  // range?
                  //std::distance(locations.begin(), range);
                  let l: Lookahead16 = n.checked_add(hops as u16).unwrap_or_else(
                    || {
                      RegexError::ExceedsLimits(k.idx()).emit();
                    }
                  );

                  if c == '(' {
                    state_ref.heads.insert(l);
                  } else {
                    state_ref.tails.insert(l);
                  }
                } else {
                  println!("{} {} ({}) {}", state.borrow().accept, position, false, n);
                }

                // start and end positions of the regex
                let m: Lookahead16 = n;
                n += locations.len() as u16;
                // check for overflow
                if n < m {
                  RegexError::ExceedsLimits(k.idx().into()).emit();
                }
              } // end iterate over lookahead_map
            } // end scope lookahead_map
          } // end if c is parens
          else {
            if self.follow_positions_map.contains_key(&Position(k.index_with_iter().into()))
            {

              // If` k.idx()` already maps to something and `k.is_lazy()`, make sure `k` also maps to
              // something. If it doesn't, make it out of lazifying `follow_positions_map[k.idx()]`.
              // Finally, set `follow` to `follow_positions_map[k]`. If `!k.is_lazy()`, just set
              // `follow` to `follow_positions_map[k.idx()]`

              let follow: VcPositionSet = // The big if statement
              if k.is_lazy() {
                // # if 1 // CHECKED algorithmic options: 7/31 self optimization works fine when
                // trim_lazy adds non-lazy greedy state, but may increase the total number of states:
                if k.is_greedy() {
                  continue;
                }


                if !self.follow_positions_map.contains_key(k) {
                  // self.follow_positions is not defined for lazy pos yet, so add lazy self.follow_positions (memoization)
                  let mut more_positions: PositionSet;
                  { // scope of follow
                    let follow = self.follow_positions(k.index_with_iter(), file!(), line!());
                    more_positions =
                      follow.iter().map(
                        |p| match p.is_ticked() {
                          true => *p,
                          false => p.set_lazy(k.lazy())
                        }
                      ).collect();
                  }
                  self.follow_positions(*k, file!(), line!()).append(&mut more_positions);
                }


                #[cfg(feature = "DEBUG")]
                {
                  println!("lazy self.follow_positions(");
                  print!("{}", k);
                  print!(" ) = {{");
                  debug_log_position_set(&*self.follow_positions_map[k].borrow(), 0);
                  println!(" }}");
                }

                self.follow_positions_map[k].clone()
              } // end if k.lazy()
              else {
                self.follow_positions_map[&Position(k.idx().into())].clone()
              };


              let mut chars: Chars = Chars::new();
              if literal {
                if c.is_alphabetic() && self.modifiers.is_set(k.idx(), Mode::i) {
                  chars.insert(c.to_uppercase());
                  chars.insert(c.to_lowercase());
                } else {
                  chars.insert(c);
                }
              } else {
                match char::from(c) {
                  '.' => {
                    // todo: These constants are ridiculous. Replace with
                    //       `Chars::new().insert('whatever')`
                    let dot_all_characters =
                    match self.modifiers.is_set(k.idx(), Mode::s) {
                      true => ALL_CHARS,         // DotAll Mode - `.` matches newlines
                      false => NON_NEWLINE_CHARS, // Excludes
                    };
                    chars |= dot_all_characters;
                  }

                  '^' => {
                    match self.modifiers.is_set(k.idx(), Mode::m) {
                      true => {
                        chars.insert(Char::from(Meta::BeginningOfLine));
                      }
                      false => {
                        chars.insert(Char::from(Meta::BeginningOfBuffer));
                      }
                    }
                  }

                  '$' => {
                    match self.modifiers.is_set(k.idx(), Mode::m) {
                      true => {
                        chars.insert(Char::from(Meta::EndOfLine));
                      }
                      false => {
                        chars.insert(Char::from(Meta::EndOfBuffer));
                      }
                    }
                  }

                  _ => {
                    if c == '[' {
                      self.idx = k.idx();
                      self.compile_list(&mut chars);
                    } else {
                      match self.escape_at(k.idx()) {
                        '0' => { // no escape at current k.idx()
                          if c.is_alphabetic() && self.modifiers.is_set(k.idx(), Mode::i) {
                            chars.insert(c);
                            chars.insert(c.toggle_case());
                          } else {
                            chars.insert(c);
                          }
                        }

                        'i' => {
                          chars.insert(Char::from(Meta::IndentBoundary));
                        }

                        'j' => {
                          chars.insert(Char::from(Meta::DedentBoundary));
                        }

                        'k' => {
                          chars.insert(Char::from(Meta::UndentBoundary));
                        }

                        'A' => {
                          chars.insert(Char::from(Meta::BeginningOfBuffer));
                        }

                        'z' => {
                          chars.insert(Char::from(Meta::EndOfBuffer));
                        }

                        'B' => {
                          match k.is_anchor() {
                            true => {
                              chars.insert(Char::from(Meta::NonWordBoundary));
                            }
                            false => {
                              chars.insert(Char::from(Meta::NonWordEnd));
                            }
                          }
                        }

                        'b' => {
                          match k.is_anchor() {
                            true => {
                              chars.insert_pair(Char::from(Meta::BeginWordBegin), Char::from(Meta::EndWordBegin));
                            }

                            false => {
                              chars.insert_pair(Char::from(Meta::BeginWordEnd), Char::from(Meta::EndWordEnd));
                            }
                          }
                        }

                        '<' => {
                          match k.is_anchor() {
                            true => {
                              chars.insert(Char::from(Meta::BeginWordBegin));
                            }
                            false => {
                              chars.insert(Char::from(Meta::BeginWordEnd));
                            }
                          }
                        }

                        '>' => {
                          match k.is_anchor() {
                            true => {
                              chars.insert(Char::from(Meta::EndWordBegin));
                            }
                            false => {
                              chars.insert(Char::from(Meta::EndWordEnd));
                            }
                          }
                        }

                        _ => {
                          c = self.parse_esc(Some(&mut chars));
                          // todo: What if 'c' is uppercase?
                          if !c.is_meta() &&
                          u8::from(c) <= b'z' &&
                          c.is_alphabetic() &&
                          self.modifiers.is_set(k.idx(), Mode::i)
                          {
                            chars.insert(c.to_uppercase());
                            chars.insert(c.to_lowercase());
                          }
                        }
                      } // end match escape_at
                    } // end c != '['
                  } // end match branch _
                } // end match char::from(c)
              } // end if not literal
              self.transition(&mut chars, follow);
            } // end if i != self.follow_positions.end()
          } // end else c is not parens
        } // end else k is not accept
      } // end for k in positions
    } // end scope of references

    let mut indices_to_remove: Vec<usize> = Vec::new();
    for (index, (_, positions)) in self.moves.iter().enumerate() {
      trim_lazy(&mut *positions.borrow_mut());
      if positions.borrow().is_empty() {
        indices_to_remove.push(index)
      }
    }
    // todo: make more efficient.
    for index in indices_to_remove.iter().rev() {
      self.moves.remove(*index);
    }

    println!("END compile_transition()");
  }


  /// Compiles things of the form `[abc]`.
  fn compile_list(&mut self, chars: &mut Chars) {
    // Don't modify `self.idx`.
    let mut idx = self.idx + 1;

    let complement: bool = self.at(idx) == '^';
    if complement {
      idx += 1;
    }

    // We use `prev` as a cursor pointing to the last character of interest.
    let mut prev: Char = Meta::BeginningOfLine.into();
    // `lo` is the bottom of a character range.
    let mut lo: Char = Meta::EndOfLine.into();
    let mut c: Char = self.at(idx);

    // for (Char c = at(loc); c != '\0' && (c != ']' || prev == Meta::BeginningOfLine); c = at( + + loc)) {
    loop {
      if !(c != '\0' && (c != ']' || prev == Meta::BeginningOfLine)) {
        break;
      }


      if c == '-' && !prev.is_meta() && lo.is_meta() {
        // Found the bottom end of a character range. (Notice this cannot happen on the first
        // character, because `prev.is_meta()` at the first character.)
        lo = prev;
      } else {
        //    [:aunum:]
        //    01234
        //    c
        // Check for posix character class expression, e.g. `[:alnum:]`
        // Look for *last* `:` first. Note loc+2 would be beyond first `:`.

        if c == '[' && self.at(idx + 1) == ':' {
          let maybe_c_loc = self.find_at(idx + 2, ':');
          if let Some(c_loc) = maybe_c_loc {
            /*
              There are two forms:
                1. `[:c:]`, which is treated as `\c`
                2. `[:alnum:]`, where "alnum" stands for any name of a Posix class.
            */
            if self.at(c_loc + 1) == ']' {
              // Check if of the form `[:c:]`. If so, treat it as `\c`.
              if c_loc == idx + 3 {
                // Point `idx` to first `:` for `parse_esc()` without throwing away the old value.
                std::mem::swap(&mut idx, &mut self.idx);
                self.idx += 1;
                c = self.parse_esc(Some(chars));
                // Restore `idx`
                std::mem::swap(&mut idx, &mut self.idx);
              } else {
                // Must be of the form `[:alnum:]`. Identify which class name is used.
                // Point `self.idx` to first character after `:` for `parse_esc()` without throwing
                // away the old value.
                std::mem::swap(&mut idx, &mut self.idx);
                self.idx += 2;

                *chars |= *self.parse_char_class();

                // Restore `self.idx`
                std::mem::swap(&mut idx, &mut self.idx);

                c = Meta::EndOfLine; // Arbitrary `Meta` variant
              }
            }
            idx = c_loc + 1; // Now points to `]`
          }
        } else if c == self.options.escape_character && !self.options.bracket_escapes {
          // An escape character with escapes in brackets enabled.
          // [\x....]
          //  c
          //  l
          // Point `self.idx` to first `:` for `parse_esc()` without throwing away the old value.
          std::mem::swap(&mut idx, &mut self.idx);
          c = self.parse_esc(Some(chars));
          // Restore `self.idx`
          std::mem::swap(&mut idx, &mut self.idx);
          // loc now points to one past the escape char.
          // [\x....]
          //   cl
        }

        // We signaled that we only found a character above by setting `c` to an arbitrary `Meta`
        // character.
        if !c.is_meta() {
          if !lo.is_meta() {
            // We already had the lower character of a character range, so `c` is the upper
            // character.
            if lo <= c {
              chars.insert_pair(lo, c);
            } else {
              RegexError::InvalidClassRange(self.idx).emit();
            }

            if self.modifiers.is_set(idx, Mode::i) {
              chars.make_case_insensitive();
            }
            // Reset search for upper character; `lo` is reset unconditionally in the outermost
            // `else`.
            c = Char::from(Meta::EndOfLine);
          } else {
            if c.is_alphabetic() && self.modifiers.is_set(idx, Mode::i) {
              chars.insert(c.to_uppercase());
              chars.insert(c.to_lowercase());
            } else {
              chars.insert(c);
            }
          }
        }

        prev = c;
        lo = Char::from(Meta::EndOfLine);
      }

      idx += 1;
      c = self.at(idx);
    }

    // If `-` is the last character in brackets, treat it as a literal.
    if !lo.is_meta() {
      chars.insert(Char::from('-'));
    }

    if complement {
      chars.flip();
    }
  }


  fn transition(&mut self, chars: &mut Chars, follow: VcPositionSet)
  {
    /*
      We have: the characters that label an edge ending in a position in the follow set.

      1. First, we find all existing transitions (moves) that are subsets of this transition, and we
         absorb them into this transition, removing them from the set of moves.
      2. Then we find any existing transitions that are super-transitions of this transition, and
         we combine this transition with whatever we find.
    */

    { // scope of `indices_to_remove`
      let mut indices_to_remove: Vec<usize> = Vec::new();
      for (index, (i_chars, i_positions)) in self.moves.iter().enumerate() {
        // Combine existing subtransitions with this transition.
        if i_positions.borrow().is_subset(&follow.borrow()) {
          *chars += *i_chars;
          indices_to_remove.push(index);
        }
      }
      for index in indices_to_remove.iter().rev() {
        self.moves.remove(*index);
      }
    }

    { // scope of more_moves
      let mut more_moves: Vec<Move> = Vec::new();
      for (i_chars, i_positions) in self.moves.iter_mut() {
        if chars.intersects(i_chars) {
          // Combine this transition with any existing super-transitions.
          let follow_ref = follow.borrow_mut();
          if ValueCell::borrow(i_positions).is_subset(&follow_ref) {
            *chars -= *i_chars;
          } else {
            // follow is not a subset of positions
            let mut positions: RefMut<PositionSet> = i_positions.deref().borrow_mut();
            if chars.is_subset(i_chars) {
              *chars -= *i_chars;
              positions.extend(&*follow_ref);
            } else {
              // Make a copy of `i`, empty original `i`.
              let (new_chars, mut new_positions): (Chars, BTreeSet<Position>) =
              (i_chars.clone(), i_positions.deref().borrow().clone());
              new_positions.extend(follow_ref.iter());
              *chars -= new_chars;
              i_chars.clear();
              more_moves.push((new_chars, ValueCell::new(new_positions)));
            }
          }
          if chars.is_empty() {
            return;
          }
        }
      }
      self.moves.append(&mut more_moves);
    }
    if !chars.is_empty() {
      self.moves.push((*chars, follow.clone()));
    }
  }

  /*


  /**
    Creates an edge labeled `c` from `state` to a newly created state, returning the new state.

    This method exists to accommodate the prefix tree optimization.
  */
  fn add_edge(&mut self, state: VcState, c: Char) -> VcState {
    let mut state_ref = state.borrow_mut();
    let target_state: VcState =
    State::with_node(
      state_ref.tnode.clone().unwrap().borrow_mut().edge[&c.into()].clone()
    );

    if self.options.insensitive_case && c.is_alphabetic() {
      let c_lower: Char = c.to_lowercase();
      let c_upper: Char = c.to_uppercase();

      state_ref.edges.insert(c_lower, (c_lower, target_state.clone()));
      state_ref.edges.insert(c_upper, (c_upper, target_state.clone()));

      self.edge_count += 2;
    }
    else {
      state_ref.edges.insert(c, (c, target_state.clone()));
      self.edge_count += 1;
    }

    target_state
  }

*/


  // endregion


  fn debug_log_follow_map(&self, indent_level: usize) {
    for (position, positions_set) in self.follow_positions_map.iter() {
      print!("{}{}{} ) = {{", " ".repeat(indent_level * 2), "follow_positions_map(", position);

      debug_log_position_set(&*ValueCell::borrow(positions_set), indent_level);
      println!(" }}");
    }
  }
}


// region Free functions

pub(crate) fn trim_lazy(positions: &mut PositionSet) {
  #[cfg(feature = "DEBUG")]
  {
    print!("BEGIN trim_lazy({{ ");
    debug_log_position_set(&positions, 0);
    println!(" }})");
  }

  // todo: This loop block feels wrong. Determine what it does and potentially rewrite.
  /*
    The original C++ code makes no sense. For example:

    ```C++
      1592:   pos->erase(--p.base());
    ```
    Since the element `p` pointed to is now erased, `p` is invalidated.
  */
  /*
      let mut p_iter_fwd = &mut positions_ref.iter();
      let mut p_iter_rev = p_iter_fwd.clone().rev(); //Positions::reverse_iterator = pos.rbegin();
      loop {
        let maybe_p = p_iter_rev.next();
        let mut p = match maybe_p {
          Some(q) if q.lazy_tag()!=0 => q,
          _ => {
            break;
          }
        };
        let lazy_p = p.lazy_tag();

        if p.is_accept() || p.is_anchor() { // CHECKED algorithmic options: 7/28 added p.anchor() {
          positions_ref.insert(p.lazy(0)); // make lazy accept/anchor a non-lazy accept/anchor

          positions_ref.remove(p_iter_fwd.next().unwrap());
          loop {
            p = match p_iter_rev.next() {
              Some(q) if !q.is_accept() && q.lazy_tag() == lazy_p => q,
              _ => {
                break;
              }
            };
            // # if 0 // CHECKED algorithmic options: set to 1 to turn lazy trimming off
            // p += 1;
            // # else
            positions_ref.remove(p_iter_fwd.next().unwrap());
            // # endif
          }
        }
        else {
          // # if 0 // CHECKED algorithmic options: 7/31
          // if (p.greedy()) {
          //   pos.insert(p.lazy(0).greedy(false));
          //   pos.erase(--p.base());
          // } else {
          //   break; // ++p;
          // }
          // # else
          if !p.is_greedy() { // stop here, greedy bit is 0 from here on
            break;
          }
          positions_ref.insert(p.lazy(0));
          positions_ref.remove(p_iter_fwd.next().unwrap()); // CHECKED 10/21 ++p;
          // # endif
        }


      // # if 0 // CHECKED algorithmic options: 7/31 but results in more states
      // while (p != pos.rend() & & p.greedy()) {
      // pos.insert(p.greedy(false));
      // pos.erase( - - p.base());
      // }
      // # endif

      // trims accept positions keeping the first only, and keeping redo (positions with accept == 0)
      let mut first_not_found: bool = true;
      for q in positions_ref.iter() {
        if q.is_accept() && q.accepts() != 0 {
          if first_not_found {
            // Keep the first accept state only.
            first_not_found = false;
          } else {
            // Erase all other accept states after the first one.
            positions_ref.remove(q);
          }
        }
      }
      */
  #[cfg(feature = "DEBUG")] {
    print!("END trim_lazy({{");
    debug_log_position_set(&positions, 0);
    println!(" }})");
  }
}


/// Makes everything in positions greedy.
// todo: make this a method on `Positions`. Requires making `Positions` a struct.
pub fn greedify(positions: &PositionSet) -> PositionSet {
  let mut new_positions: PositionSet = PositionSet::new();
  for p in positions.iter() {
    let new_position =
    match p.lazy() != 0 {
      true => *p,
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


pub fn debug_log_position_set(positions: &PositionSet, indent_level: usize) {
  //println!("{} = {{", target_set);
  print!("{}{}", " ".repeat(indent_level * 2),
         positions.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ")
  );
}


pub fn debug_display_appending(positions: &PositionSet){
  println!(
    "Appending the set {:?}",
    &positions.iter().map(|x| x.idx()).collect::<Vec<u32>>()
  );
}


// endregion


#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn parse_modifiers() {
    let parser = Parser::new("(?imsux)abc*|ghj", "");
  }

  #[test]
  fn parse_options() {
    let parser = Parser::new("", "bimopf=one.h, one.cpp, two.cpp, stdout;qrswx");
  }
}
