use std::fmt::{Display, Formatter};
use std::collections::BTreeSet;


use super::*;
use crate::valuecell::ValueCell;
use crate::relesk::parser::trim_lazy;
use crate::relesk::position::VcPositionSet;


pub type VcState = ValueCell<State>;
pub type OVcState = Option<VcState>;

type Edges = BTreeMap<Char, (Char, VcState)>; // `Edges` need to be ordered, so we use a BTreeMap.

type Lookahead16  = u16;                   //todo : Is this a Char?
type LookaheadSet = BTreeSet<Lookahead16>;


#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct State {
  is_null       : bool,
  // State::positions needs to be a Vc, because it gets put into the state table.
  pub positions : VcPositionSet,   //< Originally State subclassed Positions
  pub next      : OVcState,      //< points to next state in the list of states
  /// allocated
                               ///  depth-first by subset construction
  // pub left      : *mut State<'a>,      //< left pointer for O(log N) node insertion in the hash table
                               ///  overflow tree
  // pub right     : *mut State<'a>,      //< right pointer for O(log N) node insertion in the hash table
                               ///  overflow tree
  //pub tnode     : Option<VcNode>,       //< the corresponding tree DFA node, when applicable
  pub edges     : Edges,   //< state transitions
  pub first     : Index32,       //< index of this state in the opcode table, determined by the first
                               ///  assembly pass
  pub index     : Index32,       //< index of this state in the opcode table
  pub accept    : GroupIndex32,      //< nonzero if final state, the index of an accepted/captured
                               ///  subpattern
  pub heads     : LookaheadSet,  //< lookahead head set
  pub tails     : LookaheadSet,  //< lookahead tail set
  pub redo      : bool         //< true if this is an ignorable final state
}

impl State {

  /// A sentinel value used to avoid `Option<State>` in a billion places.
  pub fn null_state() -> VcState {
    // Initialize it to a null value
    static mut SINGLETON: *const VcState = 0 as *const VcState;
    static ONCE: std::sync::Once = std::sync::Once::new();

    unsafe {
        ONCE.call_once(|| {
          // Make it
          let singleton = VcState::new(State::default());
          singleton.borrow_mut().is_null = true;


          // Put it in the heap so it can outlive this call
          SINGLETON = std::mem::transmute(Box::new(singleton));    //  Box::new(singleton);
        });

        // Now we give out a copy of the data that is safe to use concurrently.
        (*SINGLETON).clone()
    }
  }


  pub fn with_pos(positions: VcPositionSet) -> VcState {
    VcState::new(
      State{
        //tnode: None,
        positions,
        ..State::default()
      }
    )
  }


  pub fn is_null(&self) -> bool {
    *self == Self::default()
  }


  // region Compilation Methods


  pub fn trim_lazy(&mut self){
    trim_lazy(&mut *self.positions.borrow_mut());
  }


  // endregion


}


impl Display for State{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "state<{}>", self.index)
  }
}



pub(crate) struct StateNextIterator {
  current_state: Option<VcState>
}

impl StateNextIterator{
  pub fn new(state: VcState) -> StateNextIterator {
    StateNextIterator {
      current_state: Some(state)
    }
  }
}

impl Iterator for StateNextIterator {
  type Item = VcState;

  fn next(&mut self) -> Option<Self::Item> {
    if let Some(current) = &self.current_state {
      let new_current = current.clone();
      self.current_state = new_current.borrow().next.clone();
      Some(new_current)
    } else {
      None
    }
  }
}
