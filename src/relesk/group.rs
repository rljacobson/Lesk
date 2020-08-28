#![allow(dead_code)]

/*!
  A `Group` represents a capture group in relesk. They are used during parsing and generating the
  DFA (by the `Parser`) but are not used for matching. Their primary function is to own
  information about each position within the regex that lie within the group.

  Strictly speaking, a `Group` can also be an alternation of string literals, which could be
  multiple matching groups.
*/

use std::fmt::{Display, Formatter};
//use std::cell::RefCell;

use patricia_tree::PatriciaMap;

use super::*;
use error::RegexError;
use parser::greedify;


// It is convenient to be able to specify which set to act on without taking a reference to the
// set.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum TargetSet{
  Start,
  First,
  Follow,
  Last,
  Lazy,
}

impl Display for TargetSet{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    let set_name = match self{
      TargetSet::Start => "start",
      TargetSet::First => "first",
      TargetSet::Follow => "follow",
      TargetSet::Last => "last",
      TargetSet::Lazy => "lazy"
    };
    write!(f, "{}_positions", set_name)
  }
}


#[derive(Default)]
pub struct Group{

  /// Index of the ancestral top-level group of this group
  pub idx              : GroupIndex32,
  pub first_positions  : PositionSet, //< Positions at which the first matches can occur.
  pub last_positions   : PositionSet, //< Positions at which the group match can end

  pub subpattern_endpoints: Vec<Index32>, //< Subpatterns' ending '|' or '\0'

  /// Created with the group, incremented in `parse_iterated()`
  pub lazy_index          : Lazy8,        //< ??

  /// Positions that are set as lazy. These are ultimately encoded into `Position`s (in the
  /// top-most byte) when control flow returns to the `parse()` function.
  pub lazy_set            : PositionSet,  //< Positions that are set as lazy
  pub nullable            : bool,         //< Can this group match the empty string?
  pub iteration           : Iteration16,  //< Which iteration of a repeated subpattern is it
  pub min_pattern_length  : u8,           //< Patterns after the prefix are also bound above by 8.
  pub string_trie         : PatriciaMap<GroupIndex32>,  //< String literal trie

}

impl Group {

  /// Inserts a string literal into the prefix trie
  pub fn insert_string(&mut self, string_literal: &String, idx: GroupIndex32) {
    self.string_trie.insert(string_literal, idx);
  }

  /**

  Inserts `Position(self.idx)` as an accept position into the provided `PositionSet`, setting the
  lazy byte according to `self.lazy_set`. Called only from `parser.parse()`.

  */
  pub fn append_idx_as_lazy_accepted(&mut self, position_set: &mut PositionSet){
    if self.nullable {
      if self.lazy_set.is_empty() {
        position_set.insert(Position(self.idx as u64).set_accept(true));
      }
      else {

        for l in self.lazy_set.iter() {
          position_set
              .insert(
                Position(self.idx as u64)
                .set_accept(true)
                .set_lazy(*l)
              );
        }
      }
    }
  }


  pub fn append_idx_for_last_positions(&mut self, follow_map: &mut FollowMap) {
    let last_positions = self.last_positions.clone();

    for p in last_positions.iter(){
      self.append_idx_as_lazy_accepted(
        &mut follow_map[p.idx().into()]
      );
    }
  }


  /// Extends the `TargetSet` in-place with `positions` without consuming it.
  pub fn extend_with(&mut self, target: TargetSet, positions: &PositionSet){
    self.from_target_mut(target).extend(positions.iter());
  }


  /// Same as `extend_with()` but lazifies the `source_set` first.
  pub fn extend_with_lazy(&mut self, target_set: TargetSet, source_set: &PositionSet){
    let mut lazy_source = self.lazify(&source_set);
    let position_set = self.from_target_mut(target_set);
    // As lazy_source is a copy, it may be consumed.
    position_set.append(&mut lazy_source);
  }



  /// Makes everything in positions greedy.
  pub(crate) fn greedify_own_set(&mut self, position_set: TargetSet) {
    let positions = self.from_target_mut(position_set);

    let mut new_positions: PositionSet = greedify(positions);

    /*
      CHECKED algorithmic options: 7/29 guard added: p.lazy() ? *p : p.greedy(true)
      CHECKED 10/21 pos1.insert(p.lazy(0).greedy(true));
      pos.swap(pos1);
    */
    std::mem::swap(&mut new_positions, positions);
  }




  pub fn lazify_own_set(&mut self, target_set: TargetSet){
    let positions: &PositionSet = self.from_target(target_set);
    if positions.is_empty() || self.lazy_set.is_empty() {
      return;
    }

    let mut new_positions = PositionSet::default();
    for p in positions.iter() {
      for l in self.lazy_set.iter() {
        // pos1.insert(p.lazy() ? *p : p.lazy(*l)); // CHECKED algorithmic options: only if p is not already lazy??
        new_positions.insert(p.set_lazy(*l)); // overrides lazyness even when p is already lazy
      }
    }

    let positions: &mut PositionSet = self.from_target_mut(target_set);
    std::mem::swap(&mut new_positions, positions);

  }


  /// Makes a lazy version of `positions`, which is not consumed.
  pub(crate) fn lazify(&self, positions: &PositionSet) -> PositionSet {
    //let positions = self.from_target(source_set);

    if self.lazy_set.is_empty() {
      return positions.clone();
    }
    if positions.is_empty(){
      return PositionSet::default();
    }

    let mut lazy_positions: PositionSet = PositionSet::new();

    for p in positions.iter() {
      for l in self.lazy_set.iter() {
        // pos1.insert(p.lazy() ? *p : p.lazy(*l)); // CHECKED algorithmic options: only if p is not already lazy??
        lazy_positions.insert(p.set_lazy(*l)); // overrides lazyness even when p is already lazy
      }
    }

    lazy_positions
  }


  //noinspection RsSelfConvention
  fn from_target(&self, target: TargetSet) -> &PositionSet{
    match target{
      TargetSet::First => &self.first_positions, // Parens for linter.
      TargetSet::Last => &self.last_positions,
      TargetSet::Lazy => &self.lazy_set,
      //TargetSet::Follow => self.follow_set,          // Parser member
      //TargetSet::Start => panic!("Not a valid target."),
      _t => panic!("Not a valid target: {}", _t),
    }
  }

  //noinspection RsSelfConvention
  fn from_target_mut(&mut self, target: TargetSet) -> &mut PositionSet{
    match target{
      TargetSet::First => &mut self.first_positions,
      TargetSet::Last => &mut self.last_positions,
      TargetSet::Lazy => &mut self.lazy_set,
      //TargetSet::Follow => self.follow_set,          // Parser member
      //TargetSet::Start => panic!("Not a valid target."),
      _ => panic!("Not a valid target."),
    }
  }


  /// Does `self.lazy_index.checked_add(1)` and emits an error on overflow.
  pub fn increment_lazy_index(&mut self) {
    self.lazy_index = self.lazy_index
                            .checked_add(1)
                            .unwrap_or_else(| | {
                              // overflow: too many top-level alternations (should never happen)
                              RegexError::ExceedsLimits(self.idx).emit();
                            });
  }



  // region Debug Logging
  pub fn debug_log_position_set(&self, target_set: TargetSet, indent_level: usize){
    let target = self.from_target(target_set);

    println!("{} = {{", target_set);
    for p in target.iter() {
      print!("{}", " ".repeat(indent_level*2)); // The `*2` means use two spaces.
      println!("{}", *p);
    }
    println!("}}")
  }
  // endregion

}
