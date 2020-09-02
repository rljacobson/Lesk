#![allow(dead_code)]

/*!
Compiles the data produced by the parser into a DFA.

DFA compaction:
  ```
  #[cfg(feature = "REVERSE_ORDER_EDGE_COMPACT")] -1 == reverse order edge compression (best);
  #[cfg(feature = "EDGE_COMPRESSION")]: 1 == edge compression;
  Else: no edge compression.
  ```


Edge compression reorders edges to produce fewer tests when executed in the compacted order.
For example `([a-cg-ik]|d|[e-g]|j|y|[x-z])` after reverse edge compression has only 2 edges:
```
  c1 = m.FSM_CHAR();
  if ('x' <= c1 && c1 <= 'z') goto S3;
  if ('a' <= c1 && c1 <= 'k') goto S3;
  return m.FSM_HALT(c1);
```
*/

use std::cell::{RefCell, RefMut};
use std::cmp::min;
use std::fs::{OpenOptions, File};
use std::io::Write;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::path::Path;
use std::time::Duration;

use quanta::Clock;

use super::*;
use error::RegexError;
use opcode::{
  Opcode,
  bitmasks,
  opcode_goto,
  opcode_long,
  opcode_take,
  opcode_tail
};
use options::Options;
use state::{State, VcState, StateNextIterator};
use crate::valuecell::ValueCell;
use crate::relesk::opcode::OPCODE_REDO;


type PredictMap = DefaultHashMap<u16, PredictBits8>;


static CODE_EXTENSIONS   : [&str; 4] = [".h", ".hpp", ".cpp", ".cc"];
static DFA_EXTENSIONS    : [&str; 1] = [".gv"];


pub struct Compiler<'a, 'b> {
  regex   : &'a [u8],
  // Should this just be a String?
  prefix  : Vec<u8>,   //< pattern prefix, shorter or equal to 255 bytes
  maybe_options: Option<&'b Options>,   //< Pattern compiler options


  // Compiler
  start              : VcState,
  opcode_count       : Index32,     //< number of opcodes generated
  opcode_table       : Vec<Opcode>, //< points to the opcode table
  one_pre_string     : bool,        //< true if matching one string in `prefix` without meta/anchors
  min_pattern_length : u8,          //< patterns after the prefix are also bound above by 8.


  // Match Predictor Tables
  prediction_bitmap_array: PredictMap, //< bitmap array
  predict_match_hashes   : PredictMap,
  //predict_match_hashes   : [PredictBits8; limits::HASH_MAX_IDX as usize], //< predict-match hash array
  predict_match_array    : PredictMap, //< predict-match array


  // Diagnostic/Benchmark
  opcodes_time:  Duration
}

impl<'a, 'b> Default for Compiler<'a, 'b> {
  fn default() -> Self {
    Compiler{
      regex  : &[],
      prefix : Vec::default(),
      maybe_options: None,

      start              : VcState::new(State::default()),
      opcode_count       : 0,
      opcode_table       : Vec::default(),
      one_pre_string     : false,
      min_pattern_length : 0,

      // Match Predictor Tables
      prediction_bitmap_array: DefaultHashMap::new(0xFF),
      predict_match_hashes   : DefaultHashMap::new(0xFF), //< predict-match hash array
      predict_match_array    : DefaultHashMap::new(0xFF),

      opcodes_time: Duration::default()
    }
  }
}


impl<'a, 'b> Compiler<'a, 'b> {

  pub fn new(regex: &'a [u8], start: VcState, prefix: Vec<u8>, options: &'b Options)
    -> Compiler<'a, 'b> {
      // meta/anchors
    Compiler {
      regex,
      start,
      prefix,
      maybe_options: Some(options),
      ..Compiler::default()
    }
  }




  pub(crate) fn assemble(&mut self) {

    println!("BEGIN assemble()");

    // Timing
    let timer: Clock = Clock::new();
    let start_time = timer.start();


    self.predict_match_dfa();
    self.export_dfa();
    //#[cfg(not(feature = "NO_COMPRESSION"))]
    //self.compact_dfa();
    self.encode_dfa();

    self.opcodes_time = timer.delta(start_time, timer.end());

    self.gencode_dfa();
    self.export_code();

    println!("END assemble()");
    println!("Assembly time: {}μs", self.opcodes_time.as_micros());
    println!("Opcodes: {}", self.opcode_count);
  }


  // todo: Isn't this just a debug method? If so fold it into its callee.
  fn predict_match_dfa(&mut self) {

    println!("BEGIN Pattern::predict_match_dfa()");

    self.one_pre_string = true;
    let next_state: VcState = self.start.clone();
    let mut state = next_state.clone();

    while state.borrow().accept == 0 {

      if state.borrow().edges.len() != 1 {
        self.one_pre_string = false;
        break;
      }

      {  // scope of hi, lo, next_state
        let state_clone = state.clone();
        let state_ref = state_clone.borrow();
        let (lo, (hi, next_state)) = state_ref.edges.first_key_value().unwrap();
        if !lo.is_meta() && *lo == *hi {

          if self.prefix.len() >= 255 {
            self.one_pre_string = false;
            break;
          }

          self.prefix.push(u8::from(*lo));
        }
        else {
          self.one_pre_string = false;
          break;
        }
        state = next_state.clone();
      }
      if next_state.borrow().is_null() {
        self.one_pre_string = false;
        break;
      }
    }

    { // scope of state_ref
      let state_ref = state.borrow();
      if !state_ref.is_null() && state_ref.accept != 0 && !state_ref.edges.is_empty() {
        self.one_pre_string = false;
      }
    }
    self.min_pattern_length = 0;

    //self.prediction_bitmap_array.resize(256, 0xFF);
    //self.predict_match_hashes = [0xFF; limits::HASH_MAX_IDX];
    //self.predict_match_array = [0xFF; limits::HASH_MAX_IDX];

    if !state.borrow().is_null() && state.borrow().accept == 0 {
      self.gen_predict_match(next_state.clone());

      #[cfg(feature = "DEBUG")]
      {
        let arrays = [
          (&self.prediction_bitmap_array, "prediction_bitmap_array"),
          (&self.predict_match_hashes,    "predict_match_hashes"),
          (&self.predict_match_array,     "predict_match_array"),
        ];

        for (array, name) in arrays.iter() {
          for (key, value) in array.iter() {
            if Char::from(*value).is_printable() {
              println!("{}['{}'] = {:02x}", name, key, value);
            }
            else {
              println!("{}[{:3}] = {:02x}", name, key, value);
            }
          }
        }

        /*
        for i in 0..self.prediction_bitmap_array.len() {
          if self.prediction_bitmap_array[i] != 0xFF {
            // I think we want Char::from(self.prediction_bitmap_array[i]), not `Char::from(i)`.
            // Tentatively changed
            //if (Char::from(i)).is_printable() {
            if (Char::from(self.prediction_bitmap_array[i])).is_printable() {
                println!("bit['{}'] = {:02x}", i, self.prediction_bitmap_array[i]);
            }
            else {
              println!("bit[{:3}] = {:02x}", i, self.prediction_bitmap_array[i]);
            }
          }
        }
        for i in 0..limits::HASH_MAX_IDX {
          if self.predict_match_hashes[i] != 0xFF {
            if Char::from(self.predict_match_hashes[i]).is_printable() {
              println!("predict_match_array['{}'] = {:02x}", i, self.predict_match_hashes[i]);
            }
            else {
              println!("predict_match_array[{:3}] = {:02x}", i, self.predict_match_hashes[i]);
            }
          }
        }
        for i in 0..limits::HASH_MAX_IDX {
          if self.predict_match_array[i] != 0xFF {
            if Char::from(self.predict_match_array[i]).is_printable() {
              println!("predict_match_array['{}'] = {:02x}", i, self.predict_match_array[i]);
            }
            else {
              println!("predict_match_array[{:3}] = {:02x}", i, self.predict_match_array[i]);
            }
          }
        }
        */

      }
    }

    println!("Min pattern length: {}", self.min_pattern_length);
    println!("END Pattern::predict_match_dfa()");
  }


  fn gen_predict_match(&mut self, state: VcState) {
    const LEVEL_COUNT: usize = 8;
    self.min_pattern_length = LEVEL_COUNT as u8;
    let states_maps: [RefCell<DefaultHashMap<VcState, Ranges<Hash16>>>; LEVEL_COUNT] = {
      // Rust Black Magic
      // From: https://doc.rust-lang.org/nightly/nomicon/unchecked-uninit.html

      // Create an uninitialized array of `MaybeUninit`. The `assume_init` is
      // safe because the type we are claiming to have initialized here is a
      // bunch of `MaybeUninit`s, which do not require initialization.
      type CrazyType = MaybeUninit< RefCell<DefaultHashMap<VcState, Ranges<Hash16>>> >;
      let mut x: [CrazyType; LEVEL_COUNT] = unsafe {
        MaybeUninit::uninit().assume_init()
      };

      // Dropping a `MaybeUninit` does nothing. Thus using raw pointer
      // assignment instead of `ptr::write` does not cause the old
      // uninitialized value to be dropped.
      for i in 0..LEVEL_COUNT {
        x[i] = MaybeUninit::new( RefCell::new(DefaultHashMap::default()) );
      }

      // Everything is initialized. Transmute the array to the
      // initialized type.
      unsafe { std::mem::transmute::<_, [RefCell<DefaultHashMap<VcState, Ranges<Hash16>>>; LEVEL_COUNT]>(x) }
    };


    // Bootstrap the first one, which the following loop skips.
    self.gen_predict_match_transitions(
      0,
      state.clone(),
      None,
      &mut *states_maps[0].borrow_mut()
    );
    for level in 1..LEVEL_COUNT {
      for (state, ranges) in
      states_maps[level - 1].borrow().iter() {

        self.gen_predict_match_transitions(
          level as u8,
          state.clone(),
          Some(&ranges),
          &mut *states_maps[level].borrow_mut()
        );
      }
    }

    /*
    The `min_pattern_length` is a number from 0-7. Knowing min pattern length allows:
      1. to check last char before checking intermediate, potentially skipping intermediate checks
         if no match;
      2. reject early if match string not long enough.
    */
    let constant = 1u8.checked_shl(self.min_pattern_length as u32)
                      .unwrap_or(0)
                      .wrapping_sub(1);
    self.prediction_bitmap_array.set_default(constant);
    for value in self.prediction_bitmap_array.values_mut(){
      *value &= constant;
    }
  }


  /**

  The `level`s range from 0 to 7, and `level=0` a special case. The `labels` are just the points in
  the `Ranges` object that are associated with `state`. The `states` is a map from states to
  `Ranges` specific to the given `level`.

  */
  fn gen_predict_match_transitions(
    &mut self,
    level: u8,
    state: VcState,
    maybe_labels: Option<&Ranges<Hash16>>,
    states: &mut DefaultHashMap<VcState, Ranges<Hash16> >
  )
  {
    let mut state_ref: RefMut<State> = state.borrow_mut();


    for (i_char1, (hi, i_state)) in state_ref.edges.iter_mut() {
      // (&Char, &mut (Char, VcState))
      let lo: Char = *i_char1;

      if lo.is_meta() {
        if level == 0 {
          self.min_pattern_length = 0;
        }
        break;
      }

      let next_vc: VcState = match level < 7 {
        true  => i_state.clone(),
        false => State::null_state()
      };

      let next: &State = &*next_vc.deref().borrow();
      let mut accept: bool = next.is_null() || next.accept != 0;

      if !accept {
        let mut first_flag: bool = true;
        for (c, _) in &next.edges {
          if c.is_meta() {
            if first_flag {
              *i_state = State::null_state();
            }
            accept = true;
            break;
          }
          first_flag = false;
        }

      } else if !next.is_null() && next.edges.is_empty() {
        *i_state = State::null_state();
      }

      if accept && (level == 0 || self.min_pattern_length > level) {
        self.min_pattern_length = level + 1;
      }

      // We may have changed `i_state` out from under `next`.
      let next = &*i_state.deref().borrow();


      // When `level == 0`, the predict arrays are indexed by `lo.0` instead of the hash of `lo`
      // and `i_char1`, so we specialize for the `level == 0` case.
      if level == 0 {
        for c in lo.0..=hi.0 {
          self.prediction_bitmap_array[c] &= !1;
          self.predict_match_hashes[c] &= !1;
          if accept {
            self.predict_match_array[c] &= !(1 << 7);
          }
          self.predict_match_array[c] &= !(1 << 6);
          if !next.is_null() {
            states.get_mut(i_state.clone()).insert(Char(c).hashed());
          }
        }
        // The remainder of the loop only applies for level > 0.
        continue;
      }

      // If level > 0:

      let labels = maybe_labels.unwrap();
      if level < 4 || level <= self.min_pattern_length {

        if level <= self.min_pattern_length {
          for i in lo..=*hi {
            self.prediction_bitmap_array[i.0] &= !(1 << level);
          }
        }

        for label_range in labels.as_slice().iter() {
          for label_lo in *label_range {
            for c in lo..=*hi {
              let h:Hash16  = hash_byte(label_lo, c.into());

              self.predict_match_hashes[h] &= !(1 << level);

              if level < 4 {
                if level == 3 || accept {
                  self.predict_match_array[h] &= !(1 << (7 - 2 * level));
                }
                self.predict_match_array[h] &= !(1 << (6 - 2 * level));
              }

              if !next.is_null() {
                states.get_mut(i_state.clone())
                      .insert(Char(h).hashed());
              }

            } // end iter over chars in range
          } // end iter over label_lo in labels
        } // end iter over ranges
      } // end if < level 4 || < min_pattern_length
    } // end iter over edges
  }



  /**
   Common functionality between `export_dfa()` and `export_code()`.

   `endings`       : A vector of allowed filename endings, typically file extensions.
   `content_writer`: A call-back that is provided a writer the call-back uses to write out content.

  */
  fn export_data(&self, endings: &[&str], filename: &String) -> Option<Box<dyn FnMut(&[u8])>>
  {

    // The `filename` must have one of the approved endings.
    let mut is_allowed: bool = false;
    for ending in endings.iter(){
      if filename.ends_with(ending){
        is_allowed = true;
        break;
      }
    }
    if !is_allowed{
      return None;
    }

    if filename.starts_with("stdout.") {
      let mut out = std::io::stdout();
      Some(Box::new(move |data: &[u8]| {
        #[allow(unused_must_use)]
        {
          out.write(data);
        }
      }))
    }
    else {
      let path =
      if filename.starts_with('+') {
        Path::new(&filename.as_str()[1..])
      } else {
        Path::new(&filename)
      };

      let open_result =
      if filename.starts_with('+') {
        OpenOptions::new().create(true).append(true).open(&path)
      } else {
        File::create(&path)
      };

      let mut file = match open_result {
        Ok(handle) => handle,
        Err(why) => {
          let display = path.display();
          eprintln!("Couldn't open {}: {}", display, why);
          return None;
        }
      };

      // Some(make_write(&mut file))
      Some(Box::new(move |data: &[u8]| {
        #[allow(unused_must_use)]
        {
          file.write(data);
        }
      }))
    }
  }


  fn export_dfa(&mut self) {
    let options = self.maybe_options.unwrap();

    for filename in options.filenames.iter() {
      // Get the writer for this file...
      let mut write_out = match self.export_data(&DFA_EXTENSIONS, &filename) {
        Some(f) => f,
        None => {continue;}
      };

      let fsm_str = match &options.name.is_empty() {
        true => "FSM",
        false => &options.name
      };

      write_out(
        format!(
          "digraph {} {{\n\t\trankdir=LR;\n\t\tconcentrate=true;\n\t\tnode \
              [fontname=\"ArialNarrow\"];\n\t\tedge [fontname=\"Courier\"];\n\n\t\tinit [root=true,\
              peripheries=0,label=\"{}\",fontname=\"Courier\"];\n\t\tinit -> N{:p};\n",
          fsm_str,
          &options.name,
          self.start
        ).as_bytes()
      );

      let next_iterator = StateNextIterator::new(self.start.clone());
      for state in next_iterator {
        let state_ref = state.borrow_mut();

        if state == self.start {
          write_out("\n/*START*/\t".as_bytes());
        }
        if state_ref.redo {
          write_out("\n/*REDO*/\t".as_bytes());
        } else if state_ref.accept != 0 {
          write_out(format!("\n/*ACCEPT {}*/\t", state_ref.accept).as_bytes());
        }
        for head in state_ref.heads.iter() {
          write_out(format!("\n/*HEAD {}*/\t", head).as_bytes());
        }
        for tail in state_ref.tails.iter() {
          write_out(format!("\n/*TAIL {}*/\t", tail).as_bytes());
        }

        if state != self.start     &&
        state_ref.accept == 0      &&
        state_ref.heads.is_empty() &&
        state_ref.tails.is_empty()
        {
          // RJ: Q: What is the point of this?
          //     A: The condition says none of the other headings above printed. This we print
          //        a generic heading.
          write_out("\n/*STATE*/\t".as_bytes());
        }

        write_out(format!("N{:p} [label=\"", &*state_ref).as_bytes());

        // # ifdef DEBUG
        #[cfg(feature = "DEBUG")]
        {
          // Heuristics for line lengths
          let mut k: usize = 1;
          let n: usize = ((state_ref.positions.borrow().len() as f64).sqrt() + 0.5) as usize;
          let mut sep = String::from("");

          for i in state_ref.positions.borrow().iter() {
            write_out(format!("{}", sep).as_bytes());
            if i.is_accept() {
              write_out(format!("({})", i.accepts()).as_bytes());
            }
            else {
              if i.is_iterable() {
                write_out(format!("{}.", i.iterations()).as_bytes());
              }
              write_out(format!("{}", i.idx()).as_bytes());
            }

            if i.is_lazy() {
              write_out(format!("?{}", i.is_lazy()).as_bytes());
            }

            if i.is_anchor() {
              write_out("^".as_bytes());
            }

            if i.is_greedy() {
              write_out("!".as_bytes());
            }

            if i.is_ticked() {
              write_out("'".as_bytes());
            }

            if k % n != 0 {
              sep = String::from(" ");
            } else {
              sep = String::from("\\n");
            }

            k += 1;
          }

          if (state_ref.accept != 0 && !state_ref.redo) ||
          !state_ref.heads.is_empty()               ||
          !state_ref.tails.is_empty()
          {
            write_out("\\n".as_bytes());
          }
          // # endif
        }

        if state_ref.accept != 0 && !state_ref.redo {
          write_out(format!("[{}]", state_ref.accept).as_bytes());
        }

        for tail in state_ref.tails.iter() {
          write_out(format!("{}>", tail).as_bytes());
        }

        for head in state_ref.heads.iter() {
          write_out(format!("<{}", head).as_bytes());
        }

        if state_ref.redo {
          write_out("\",style=dashed,peripheries=1];\n".as_bytes());
        } else if state_ref.accept != 0 {
          write_out(format!("\",peripheries=2];\n").as_bytes());
        } else if !state_ref.heads.is_empty() {
          write_out("\",style=dashed,peripheries=2];\n".as_bytes());
        } else {
          write_out("\"];\n".as_bytes());
        }

        for (i_char1, (i_char2, i_state)) in state_ref.edges.iter() {
          // # if REVERSE_ORDER_EDGE_COMPACT == -1

          #[cfg(feature = "REVERSE_ORDER_EDGE_COMPACT")]
          let lo: Char = *i_char1;
          #[cfg(feature = "REVERSE_ORDER_EDGE_COMPACT")]
          let hi: Char = *i_char2;
          // # else
          #[cfg(not(feature = "REVERSE_ORDER_EDGE_COMPACT"))]
          let hi: Char = *i_char1;
          #[cfg(not(feature = "REVERSE_ORDER_EDGE_COMPACT"))]
          let lo: Char = *i_char2;
          // # endif

          if !lo.is_meta() {
            write_out(format!("\t\tN{:p} -> N{:p} [label=\"", &*state, *i_state).as_bytes());

            // todo: Why are the options here different from `print_char()`?
            write_out(lo.escaped().as_bytes());

            if lo != hi {
              // Express as range.
              write_out("-".as_bytes());
              write_out(hi.escaped().as_bytes());
            }

            write_out("\"];\n".as_bytes());
          }
          else {

            for lo in lo..=hi {
              // todo: Which address are we printing? That is, do we need the `*`'s?
              write_out(format!("\t\tN{:p} -> N{:p} [label=\"{}\",style=\"dashed\"];\n",
                                state, i_state, lo).as_bytes());
            }

          }
        }

        if state_ref.redo {
          write_out(format!("\t\tN{:p} -> R{:p};\n\t\tR{:p} [peripheries=0,label=\"redo\"];\n",
                            &*state_ref, &*state_ref, &*state_ref).as_bytes());
        }

        write_out("}\n".as_bytes());

      }
    }
  }


  fn export_code(&mut self) {
    let options = self.maybe_options.unwrap();

    if self.opcode_count == 0 || options.optimize_fsm {
      return;
    }

    for filename in &options.filenames{
      // Get the writer for this file...
      let mut write_out = match self.export_data(&CODE_EXTENSIONS, &filename) {
        Some(f) => f,
        None => {continue;}
      };

      write_out("#ifndef REFLEX_CODE_DECL\n#include <reflex/pattern.h>\n#define \
          REFLEX_CODE_DECL reflex::Pattern::Opcode\n#endif\n\n".as_bytes());

      let namespaces = options.z_namespace.split("::").into_iter();
      for name in namespaces {
        write_out(format!("namespace {} {{\n", name).as_bytes());
      }

      write_out(
        format!(
          "extern REFLEX_CODE_DECL reflex_code_{}[{}] =\n{{\n",
          match &options.name.is_empty() {
            true => "FSM",
            false => &options.name
          },
          self.opcode_count
        ).as_bytes()
      );

      for mut i in 0..self.opcode_count {
        let mut opcode: Opcode = self.opcode_table[i as usize];
        let lo: Char = opcode.lo();
        let hi: Char = opcode.hi();

        write_out(format!("  0x{:08}X, // {}: ", opcode, i).as_bytes());

        if opcode.is_redo() {
          write_out("REDO\n".as_bytes());
        } else if opcode.is_take() {
          write_out(format!("TAKE {}\n", opcode.long_idx()).as_bytes());
        } else if opcode.is_tail() {
          write_out(format!("TAIL {}\n", opcode.long_idx()).as_bytes());
        } else if opcode.is_head() {
          write_out(format!("HEAD {}\n", opcode.long_idx()).as_bytes());
        } else if opcode.is_halt() {
          write_out("HALT\n".as_bytes());
        } else {
          let mut index: Index32 = opcode.idx();

          if index == bitmasks::HALT {
            write_out("HALT ON ".as_bytes());
          } else {
            if index == bitmasks::LONG {
              i += 1;
              opcode = self.opcode_table[i as usize];
              index = opcode.long_idx();
              write_out(format!("GOTO\n  0x{:08}X, // {}:  FAR {} ON ", opcode, i, index).as_bytes());
            } else {
              write_out(format!("GOTO {} ON ", index).as_bytes());
            }
          }

          if !lo.is_meta() {
            write_out(format!("\\\\x{:02x}", lo.0).as_bytes());

            if lo != hi {
              // Express as range
              write_out("-".as_bytes());
              write_out(format!("\\\\x{:02x}", hi.0).as_bytes());
            }
          }
          else {
            write_out(format!("{}", lo).as_bytes());
          }

          write_out("\n".as_bytes());
        }
      }

      write_out("};\n\n".as_bytes());

      if options.predict_match_array {
        self.write_predictor(&mut write_out);
      }

      let namespaces = options.z_namespace.split("::").into_iter();
      for name in namespaces {
        write_out(format!("}} // namespace {}\n\n", name).as_bytes());
      }
    }
  }


  fn write_predictor(&self, write_out: &mut Box<dyn FnMut(&[u8])>) {
    let options = self.maybe_options.unwrap();

    // Compute the length of the predictor data array. See `Pattern::init()`.
    let array_length = 2 + self.prefix.len() +
    ((self.min_pattern_length > 1 && self.prefix.len() == 0) as usize) * 256 +
    ((self.min_pattern_length > 0) as usize) * limits::HASH_MAX_IDX as usize;
    write_out(
      format!(
        "extern reflex::Pattern::Pred reflex_pred_{}[{}] = {{",
        &options.name,
        array_length
      ).as_bytes()
    );

    // Used locally below.
    fn write_array(array: &PredictMap, write_out: &mut Box<dyn FnMut(&[u8])
    >) {
      for (index, value) in array.iter() {
        write_out(
          format!("{}{:3},",
                  match index & 0xF {
                    0 => "\n  ",
                    _ => ""
                  },
                  !*value
          ).as_bytes()
        );
      }
    }

    write_out(format!(
      "\n  {:3},{:3},",
      (self.prefix.len() as u8),
      (  (&self.min_pattern_length | ((self.one_pre_string as u8) << 4)) as u8 )
    ).as_bytes());

    write_out(self.prefix.as_slice());
    write_out(",".as_bytes());

    if self.min_pattern_length > 1 && self.prefix.len() == 0 {
      write_array(&self.prediction_bitmap_array, write_out);
    }

    if self.min_pattern_length >= 4 {
      write_array(&self.predict_match_hashes, write_out);
    }
    else if self.min_pattern_length > 0 {
      write_array(&self.predict_match_array, write_out);
    }

    write_out("\n}};\n\n".as_bytes());
  }


  fn encode_dfa(&mut self) {
    self.opcode_count = 0;

    for state in StateNextIterator::new(self.start.clone()) {

      let mut state_ref = state.borrow_mut();
      state_ref.accept = min(limits::ACCEPT_MAX, state_ref.accept);
      state_ref.index = self.opcode_count;
      state_ref.first = self.opcode_count;

      // Used to record whether or not we
      let mut captured_covered: bool = false;

      let (lo, _hi): (Char, Char) =
      self.scan_and_count_edges(
        &state_ref,
        |
          lo: &mut Char,
          hi: &mut Char,
          (_i_char1, (i_char2, _i_state)): (&Char, &(Char, VcState))
        | {
          let mut count: u32 = 1;

          #[cfg(feature = "REVERSE_ORDER_EDGE_COMPACT")] {
            captured_covered = (*hi > 0xFF);
            if lo.is_meta() {
              count += (i_char2.0 - lo.0) as u32;
            }
          }
          #[cfg(not(feature = "REVERSE_ORDER_EDGE_COMPACT"))] {
            if lo.is_meta() {
              count += (hi.0 - i_char2.0) as u32;
              if i_char2.0 == 0 {
                captured_covered = true;
                *lo = *hi; // Undo decrement `lo`
              }
            }
          }

          count
        }
      );

      // add final dead state (opcode: HALT) only when needed, i.e. skip dead
      // state if all chars 0-255 are already covered
      #[cfg(feature = "REVERSE_ORDER_EDGE_COMPACT")]
      {
        // RJ: Question: Shouldn't this inequality be strict?
        //     Answer:   No, because hi is incremented at the bottom of the loop
        //               and so stops at 256.
        if !captured_covered {
          state_ref.edges.insert(hi, (Char(0xFF), State::null_state()));
          self.opcode_count += 1;
        }
      }
      #[cfg(not(feature = "REVERSE_ORDER_EDGE_COMPACT"))]
      {
        if !captured_covered {
          state_ref.edges.insert(lo, (Char(0), State::null_state()));
          self.opcode_count += 1;
        }
      }

      self.opcode_count += (state_ref.heads.len() +
      state_ref.tails.len() +
      ((state_ref.accept > 0 || state_ref.redo) as usize))
      as Index32;

      if !valid_goto_index(self.opcode_count) {
        let idx = self.regex.len() as Index32;
        RegexError::ExceedsLimits(idx).emit();
      }

    }

    if self.opcode_count > bitmasks::LONG {
      // over 64K opcodes: use 64-bit GOTO LONG opcodes
      self.opcode_count = 0;

      for state in StateNextIterator::new(self.start.clone()) {
        let mut state_ref = state.borrow_mut();

        state_ref.index = self.opcode_count;

        self.scan_and_count_edges(
          &state_ref,
          |
            lo: &mut Char,
            hi: &mut Char,
            (_i_char1, (i_char2, i_state)): (&Char, &(Char, VcState))
          |
          {
            let i_state_ref = i_state.borrow_mut();
            let mut sum: u32 = 0;

            // Check for null_ptr sentinel.
            // use 64-bit jump opcode if forward jump determined by previous loop
            // is beyond 32K or backward jump is beyond 64K
            let use_64bit: bool =
            !i_state_ref.is_null() &&
            (
              (i_state_ref.first > state_ref.first &&
              i_state_ref.first >= bitmasks::LONG / 2) ||
              i_state_ref.index >= bitmasks::LONG
            );
            let multiplier =
            match use_64bit {
              true  => 2,
              false => 1
            };

            sum += multiplier;
            if lo.is_meta() {
              #[cfg(feature = "REVERSE_ORDER_EDGE_COMPACT")]
              { sum += multiplier * (i_char2.0 - lo.0) as u32; }
              #[cfg(not(feature = "REVERSE_ORDER_EDGE_COMPACT"))]
              { sum += multiplier * (hi.0 - i_char2.0) as u32; }
            }

            sum
          }
        );

        self.opcode_count += (state_ref.heads.len() +
        state_ref.tails.len() +
        ((state_ref.accept > 0 || state_ref.redo) as usize)) as u32;
        if !valid_goto_index(self.opcode_count) {
          let idx = self.regex.len() as GroupIndex32;
          RegexError::ExceedsLimits(idx).emit();

        }
      } // end outer for
    } //end if self.nop_ > Const::LONG.


    eprintln!("Making opcode table with {} edges.", self.opcode_count);
    let mut opcode_table = Vec::with_capacity(self.opcode_count as usize);

    for state in StateNextIterator::new(self.start.clone()) {
      let state_ref = state.borrow_mut();

      if state_ref.redo {
        opcode_table.push(OPCODE_REDO);
      } else if state_ref.accept > 0 {
        opcode_table.push(opcode_take(state_ref.accept));
      }

      for state_vector in [&state_ref.tails, &state_ref.heads].iter(){
        for i in state_vector.iter() {
          if !valid_lookahead_index(*i as Index32) {
            RegexError::ExceedsLimits(self.regex.len() as GroupIndex32).emit();
          }
          opcode_table.push(opcode_tail(*i as Index32));
        }
      }

      for (i_char1, (i_char2, i_state)) in state_ref.edges.iter().rev() {

        #[cfg(feature = "REVERSE_ORDER_EDGE_COMPACT")]
        let mut lo: Char = *i_char1;
        #[cfg(feature = "REVERSE_ORDER_EDGE_COMPACT")]
        let hi: Char = *i_char2;
        #[cfg(not(feature = "REVERSE_ORDER_EDGE_COMPACT"))]
        let hi: Char = *i_char1;
        #[cfg(not(feature = "REVERSE_ORDER_EDGE_COMPACT"))]
        let mut lo: Char = *i_char2;

        let target_first: Index32;
        let target_index: Index32;
        { // Scope of `i_state_ref`
          let i_state_ref = i_state.borrow();
          match i_state_ref.is_null() {
            true => {
              target_first = limits::IMAX_IDX;
              target_index = limits::IMAX_IDX;
            }
            false => {
              target_first = i_state_ref.first;
              target_index = i_state_ref.index;
            }
          }
        }
        // Local helper function
        let mut push_opcode = |lo: Char, hi: Char| {
          if target_index == limits::IMAX_IDX {
            // occurs when i_state == null.
            opcode_table.push(opcode_goto(lo, hi, bitmasks::HALT));
          } else if self.opcode_count > bitmasks::LONG &&
          ((target_first > state_ref.first &&
          target_first >= bitmasks::LONG / 2) ||
          target_index >= bitmasks::LONG)
          {
            opcode_table.push(opcode_goto(lo, hi, bitmasks::LONG));
            opcode_table.push(opcode_long(target_index));
          } else {
            opcode_table.push(opcode_goto(lo, hi, target_index));
          }
        };

        if lo.is_meta() {
          loop {
            push_opcode(lo, lo);
            lo += 1;

            #[cfg(feature = "REVERSE_ORDER_EDGE_COMPACT")]
            if !(lo <= hi) { break; }
            #[cfg(not(feature = "REVERSE_ORDER_EDGE_COMPACT"))]
            if lo <= hi { break; }
          } // end loop
        }
        else {
          push_opcode(lo, hi);
        } // end lo.is_meta()..else...
      } // end iterate over edges
    } // end NextStateIterator
  }



  fn scan_and_count_edges<F>(
    &mut self,
    state_ref: &State,
    mut f: F) -> (Char, Char)  // todo: neither usage uses the return value.
    where F: FnMut(&mut Char, &mut Char, (&Char, &(Char, VcState))) -> u32
  {
    #[cfg(feature = "REVERSE_ORDER_EDGE_COMPACT")]
    let next_iter = state_ref.edges.iter();

    #[cfg(not(feature = "REVERSE_ORDER_EDGE_COMPACT"))]
    let next_iter = state_ref.edges.iter().rev();

    let mut lo: Char = Char(0xFF);
    let mut hi: Char = Char(0);
    for item in next_iter {
      #[cfg(feature = "REVERSE_ORDER_EDGE_COMPACT")]
      {
        lo = *item.0;
        if lo == hi {
          hi = (item.1).0 + 1;
        }
      }
      #[cfg(not(feature = "REVERSE_ORDER_EDGE_COMPACT"))]
      {
        hi = *item.0;
        if lo == hi {
          lo = (item.1).0 - 1;
        }
      }

      // item = (i_char1, (i_char2, i_state))
      self.opcode_count += f(&mut lo, &mut hi, item);
    }
    (lo, hi)
  }


  /**
  Generates threaded code instead of bytecode for the FSM. Example from the docs:

  ```c
  void reflex_code_FSM(reflex::Matcher& m)
  {
    int c0 = 0, c1 = 0;
    m.FSM_INIT(c1);
  S0:
    c1 = m.FSM_CHAR();
    if (97 <= c1 && c1 <= 122) goto S5;
    if (c1 == 95) goto S5;
    if (65 <= c1 && c1 <= 90) goto S5;
    if (48 <= c1 && c1 <= 57) goto S5;
    return m.FSM_HALT(c1);
  S5:
    m.FSM_TAKE(1);
    c1 = m.FSM_CHAR();
    if (97 <= c1 && c1 <= 122) goto S5;
    if (c1 == 95) goto S5;
    if (65 <= c1 && c1 <= 90) goto S5;
    if (48 <= c1 && c1 <= 57) goto S5;
    return m.FSM_HALT(c1);
  }
  ```
  */
  fn gencode_dfa(&mut self) {
    let options = self.maybe_options.unwrap();

    if !&options.optimize_fsm {
      return;
    }

    for filename in options.filenames.iter() {
      // Get the writer for this file...
      let mut write_out = match self.export_data(&CODE_EXTENSIONS, &filename) {
        Some(f) => f,
        None => {continue;}
      };


      write_out(r##"#include <reflex/matcher.h>

            #if defined(OS_WIN)
            #pragma warning(disable:4101 4102)
            #elif defined(__GNUC__)
            #pragma GCC diagnostic ignored "-Wunused-variable"
            #pragma GCC diagnostic ignored "-Wunused-label"
            #elif defined(__clang__)"
            #pragma clang diagnostic ignored "-Wunused-variable"
            #pragma clang diagnostic ignored "-Wunused-label"
            #endif
            "##.as_bytes());

      let namespaces = options.z_namespace.split("::").into_iter();
      for name in namespaces {
        write_out(format!("namespace {} {{\n", name).as_bytes());
      }
      { // Scope of name
        let name = match &options.name.is_empty() {
          true  => "FSM",
          false => &options.name
        };
        write_out(
          format!(
            r#"void reflex_code_{}(reflex::Matcher& m)
              {{
                int c0 = 0, c1 = 0;
                m.FSM_INIT(c1);
              "#,
            name,
          ).as_bytes()
        );
      }
      let next_iterator = StateNextIterator::new(self.start.clone());
      for state in next_iterator {
        let state_ref = state.borrow_mut();

        write_out(format!("\nS{}:\n", state_ref.index).as_bytes());
        // Only for the self.start state:
        if state == self.start {
          write_out("  m.FSM_FIND();\n".as_bytes());
        }

        if state_ref.redo {
          write_out("  m.FSM_REDO();\n".as_bytes());
        } else if state_ref.accept > 0 {
          write_out(format!("  m.FSM_TAKE({});\n", state_ref.accept).as_bytes());
        }

        for i in &state_ref.tails {
          write_out(format!("  m.FSM_TAIL({});\n", i).as_bytes());
        }
        for i in &state_ref.heads {
          write_out(format!("  m.FSM_HEAD({});\n", i).as_bytes());
        }

        if !state_ref.edges.is_empty() &&
           *state_ref.edges.keys().next().unwrap() == Meta::DedentBoundary
        {
          let (_, (_, last_edge_target)) = state_ref.edges.last_key_value().unwrap();
          let index = last_edge_target.borrow().index;
          write_out(format!("  if (m.FSM_DENT()) goto S{};\n", index).as_bytes());
        }

        // if we need to keep the previous character in c0
        let mut keep_previous_char_in_c0: bool = false;
        // if we need to read a character into c1
        let mut peek_next_char_into_c1: bool = false;

        let mut state_edges_iter = state_ref.edges.iter().rev().peekable();

        // We cannot use `for` because a `for` borrows the iterator for the duration of the loop.
        //for (i_char1, (i_char2, i_state)) in state_edges_iter{
        loop {
          let i_char1: &Char;
          let i_char2: &Char;
          let i_state: &ValueCell<State>;
          if let Some( (ic1, (ic2, is))) = state_edges_iter.next() {
            i_char1 = ic1;
            i_char2 = ic2;
            i_state = is;
          } else {
            break;
          }

          let mut lo : Char;
          let hi     : Char;
          #[cfg(feature = "REVERSE_ORDER_EDGE_COMPACT")]
          {
            lo = *i_char1;
            hi = *i_char2;
          }
          #[cfg(not(feature = "REVERSE_ORDER_EDGE_COMPACT"))]
          {
            hi = *i_char1;
            lo = *i_char2;
          }

          if !lo.is_meta() {
            let mut target_index: Index32 = limits::IMAX_IDX;

            {// scope of i_state_ref
              let i_state_ref = i_state.borrow();
              if !i_state_ref.is_null() {
                target_index = i_state_ref.index;
              }
            }

            if target_index == limits::IMAX_IDX {
              match &state_edges_iter.peek() {

                None => {
                  break;
                }

                Some((_, (c, _))) if c.is_meta() => {
                  break;
                }

                _ => { /* pass */ }

              }
            }

            peek_next_char_into_c1 = true;
          } else {

            loop {
              // branchless version:
              peek_next_char_into_c1   =   lo == Meta::EndOfBuffer  ||
              lo == Meta::EndOfLine;
              keep_previous_char_in_c0 = !peek_next_char_into_c1    &&
                                          (lo == Meta::EndWordEnd   ||
                                           lo == Meta::BeginWordEnd ||
                                           lo == Meta::NonWordEnd);
              peek_next_char_into_c1   = peek_next_char_into_c1     ||
              keep_previous_char_in_c0;

              check_dfa_closure(
                i_state.clone(),
                2,
                &mut peek_next_char_into_c1,
                &mut keep_previous_char_in_c0
              );

              lo += 1;
              if lo > hi {
                break;
              }
            }

          }
        }

        let mut second_peek_next_char_into_c1: bool = peek_next_char_into_c1;
        let mut in_elif_branch: bool = false;

        #[cfg(not(feature = "REVERSE_ORDER_EDGE_COMPACT"))]
        let mut state_edges_iter = state_ref.edges.iter().rev().peekable();
        #[cfg(feature = "REVERSE_ORDER_EDGE_COMPACT")]
        let mut state_edges_iter = state_ref.edges.iter().peekable();

        // for (i_char1, (i_char2, i_state)) in state_edges_iter {
        loop{
          let i_char1: &Char;
          let i_char2: &Char;
          let i_state: &ValueCell<State>;
          if let Some((i1, (i2, is))) = state_edges_iter.next() {
            i_char1 = i1;
            i_char2 = i2;
            i_state = is;
          } else {
            break;
          }


          let lo: Char;
          let hi: Char;
          #[cfg(feature = "REVERSE_ORDER_EDGE_COMPACT")]
          {
            lo = *i_char1;
            hi = *i_char2;
          }
          #[cfg(not(feature = "REVERSE_ORDER_EDGE_COMPACT"))]
          {
            hi = *i_char1;
            lo = *i_char2;
          }



          let mut target_index: Index32 = limits::IMAX_IDX;
          {// Scope of `i_state_ref`
            let i_state_ref = i_state.borrow_mut();
            if !i_state_ref.is_null() {
              target_index = i_state_ref.index;
            }
          }
          if second_peek_next_char_into_c1 {
            if keep_previous_char_in_c0 {
              write_out("  c0 = c1, c1 = m.FSM_CHAR();\n".as_bytes());
            } else {
              write_out("  c1 = m.FSM_CHAR();\n".as_bytes());
            }
            second_peek_next_char_into_c1 = false;
          }

          if !lo.is_meta() {
            if target_index == limits::IMAX_IDX {
              match &state_edges_iter.peek() {
                None => {
                  break;
                }
                Some((_, (c, _))) if c.is_meta() => {
                  break;
                }
                _ => { /* pass */ }
              }
            }

            if lo == hi {
              write_out(format!("  if (c1 == {}", lo.to_printable()).as_bytes());
              write_out(")".as_bytes());
            }
            else if hi.0 == 0xFF {
              write_out(format!("  if ({}", lo.to_printable()).as_bytes());
              write_out(" <= c1)".as_bytes());
            }
            else {
              write_out(format!("  if ({}", lo.to_printable()).as_bytes());
              write_out(format!(" <= c1 && c1 <= {})", hi.to_printable()).as_bytes());
            }

            if target_index == limits::IMAX_IDX {

              if peek_next_char_into_c1 {
                write_out(" return m.FSM_HALT(c1);\n".as_bytes());
              } else {
                write_out(" return m.FSM_HALT();\n".as_bytes());
              }

            }
            else {
              // todo: Goto? Really?
              write_out(format!(" goto S{};\n", target_index).as_bytes());
            }

          }
          else {
            for lo in lo..=hi {
              let fsm_meta_args: &str;
              fsm_meta_args = match lo {
                | Meta::EndOfBuffer
                | Meta::EndOfLine => "c1",

                | Meta::EndWordEnd
                | Meta::BeginWordEnd
                | Meta::NonWordEnd => "c0, c1",

                _ => ""
              };

              write_out("  ".as_bytes());

              if in_elif_branch {
                write_out("else ".as_bytes());
              }

              write_out(format!("if (m.FSM_META_{}({})) {{\n", lo, fsm_meta_args).as_bytes());

              gencode_dfa_closure(&mut write_out, i_state.clone(), 2, peek_next_char_into_c1);

              write_out("  }}\n".as_bytes());
              in_elif_branch = true;

            } // end loop
          }
        }

        if peek_next_char_into_c1 {
          write_out("  return m.FSM_HALT(c1);\n".as_bytes());
        } else {
          write_out("  return m.FSM_HALT();\n".as_bytes());
        }
      }

      write_out("}\n\n".as_bytes());

      if options.predict_match_array {
        self.write_predictor(&mut write_out);
      }

      let namespaces = options.z_namespace.split("::").into_iter();
      for name in namespaces {
        write_out(format!("}} // namespace {}\n\n", name).as_bytes());
      }
    }

  }



}


// region Free Functions

/*
pub fn hash_pos(pos: &PositionSet) -> usize {
  let mut h: usize = 0;
  for i in pos.iter() {
    h += (i.0 ^ (i.0 >> 24 as u64)) as usize; // (Position(*i).iter() << 4) unique hash for up to 16 chars iterated (abc...p){iter }
  }
  return h;
}

*/


pub fn gencode_dfa_closure(write_out: &mut Box<dyn FnMut(&[u8])>, state: VcState, nest_level: usize,
                       peek_next_char_into_c1: bool)
{
  let state_ref: &mut State = &mut state.borrow_mut();

  if state_ref.redo {
    if peek_next_char_into_c1 {
      write_out(format!("{:indent$}m.FSM_REDO(c1);\n", "", indent = 2 * nest_level).as_bytes());
    } else {
      write_out(format!("{:indent$}m.FSM_REDO();\n", "", indent = 2 * nest_level).as_bytes());
    }
  }
  else if state_ref.accept > 0 {
    if peek_next_char_into_c1 {
      write_out(
        format!(
          "{:indent$}m.FSM_TAKE({}, c1);\n",
          "", state_ref.accept,
          indent = 2 * nest_level
        ).as_bytes()
      );
    } else {
      write_out(
        format!(
          "{:indent$}m.FSM_TAKE({});\n",
          "", state_ref.accept,
          indent = 2 * nest_level
        ).as_bytes()
      );
    }
  }
  for i in state_ref.tails.iter() {
    write_out(
      format!(
        "{:indent$}m.FSM_TAIL({});\n",
        "", i,
        indent = 2 * nest_level
      ).as_bytes()
    );
  }
  // RJ: ?!
  if nest_level > 5 {
    return;
  }
  for (i_char1, (i_char2, i_state)) in state_ref.edges.iter().rev() {
    #[cfg(feature = "REVERSE_ORDER_EDGE_COMPACT")]
    let mut lo = i_char1;
    #[cfg(feature = "REVERSE_ORDER_EDGE_COMPACT")]
    let hi = i_char2;

    #[cfg(not(feature = "REVERSE_ORDER_EDGE_COMPACT"))]
    let hi = i_char1;
    #[cfg(not(feature = "REVERSE_ORDER_EDGE_COMPACT"))]
    let lo = i_char2;


    if lo.is_meta() {
      let mut in_elif_branch: bool = false;

      for lo in *lo..=*hi {
        let arguments =
        match lo {

          | Meta::EndOfBuffer
          | Meta::EndOfLine => "c1",

          | Meta::EndWordEnd
          | Meta::BeginWordEnd
          | Meta::NonWordEnd => "c0, c1",

          _ => ""
        };

        write_out(format!("{:indent$}", "", indent = 2 * nest_level).as_bytes());

        if in_elif_branch {
          write_out(format!("else ").as_bytes());
        }

        write_out(format!("if (m.FSM_META_{}({})) {{\n", lo, arguments).as_bytes());

        gencode_dfa_closure(write_out, i_state.clone(), nest_level + 1, peek_next_char_into_c1);

        write_out(format!("{:indent$}}}\n", "", indent = 2 * nest_level).as_bytes());

        in_elif_branch = true;
      }
    }
  }
}



pub fn check_dfa_closure(state: VcState, nest: i32, peek: &mut bool, prev: &mut bool) {
  if nest > 5 {
    return;
  }

  let state_ref = state.borrow_mut();
  let state_iter = state_ref.edges.iter().rev();

  for (i_char1, (i_char2, i_state)) in state_iter {
    // # if REVERSE_ORDER_EDGE_COMPACT == -1
    let lo: Char;
    let hi: Char;

    #[cfg(feature = "REVERSE_ORDER_EDGE_COMPACT")]
    {
      lo = *i_char1;
      hi = *i_char2;
    }
    #[cfg(not(feature = "REVERSE_ORDER_EDGE_COMPACT"))]
    {
      hi = *i_char1;
      lo = *i_char2;
    }

    if lo.is_meta() {
      for lo in lo..=hi {
        *peek = (lo == Meta::EndOfBuffer) || (lo == Meta::EndOfLine);
        *prev = !*peek && (lo == Meta::EndWordEnd || lo == Meta::BeginWordEnd || lo == Meta::NonWordEnd);
        *peek = *peek || *prev;
        check_dfa_closure(i_state.clone(), 2, peek, prev);
      }

    }
  }
}


pub fn valid_goto_index(index: Index32) -> bool {
  return index <= limits::GOTO_MAX_IDX;
}

pub fn valid_take_index(index: Index32) -> bool {
  return index <= limits::ACCEPT_MAX;
}

pub fn valid_lookahead_index(index: Index32) -> bool {
  return index <= limits::LOOKAHEAD_MAX_IDX;
}



/**

The inverse of `Char::hashed()`. The input `h` and output are `usize` only because it is used as
an index into an array more that it is used as a `Hash16`.

*/
pub fn hash_byte(h: Hash16, b: u8) -> Hash16 {
  // `h` is a `usize`
  // data.
  return (((h as Hash16) << 3) ^ b as Hash16) & (limits::HASH_MAX_IDX as Hash16 - 1);
}


//endregion
