#![allow(dead_code)]

/**
  Wraps an `Rc<RefCell<T>>`, transparently allowing borrows of the underlying `T`, in a
  `ValueCell<T>`. The `ValueCell<T>` is suitable for storing in a `HashMap`, as it hashes to the
  underlying `Rc`'s pointer value.

*/

use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::cell::{RefCell, Ref, RefMut};
use std::ops::Deref;
// use std::borrow::{Borrow, BorrowMut};
use std::fmt::{Pointer, Formatter};

fn hash_rc<T, H: Hasher>(rc: Rc<T>, state: &mut H) {
  let raw_ptr = Rc::into_raw(rc);
  raw_ptr.hash(state);
  // Convert back to Rc to prevent memory leak.
  let _ = unsafe {Rc::from_raw(raw_ptr)};
}


#[derive(Debug, Eq)]
pub struct ValueCell<T> {
  value: Rc<RefCell<T>>,
}


impl<T> ValueCell<T> {
  /// Constructs a new [`ValueCell<T>`](struct.ValueCell.html)
  /// wrapping a `T` in an `Rc<RefCell<T>>`.
  pub fn new(value: T) -> Self {
    ValueCell {
      value: Rc::new(RefCell::new(value))
    }
  }

  /// Returns a clone of the wrapped `Rc<T>` without consuming `self`.
  pub fn get_cloned(&self) -> Rc<RefCell<T>> {
    self.value.clone()
  }

  pub fn borrow_mut(&self) -> RefMut<T> {
    self.value.deref().borrow_mut()
  }

  pub fn borrow(&self) -> Ref<T> {
    self.value.deref().borrow()
  }

  pub fn unwrap_rc(&self) -> Rc<RefCell<T>> {
    self.value.clone()
  }

  pub fn try_unwrap<'a>(self) -> Result<T, ValueCell<T>> {
    Rc::try_unwrap(self.value)
        .and_then(| ref_cell | Ok(ref_cell.into_inner()))
        .or_else(| x |{
          Err(ValueCell{ value: x})
        })
  }

  /*
  pub fn get(&self) -> Option<Ref<T>> {
    // There is no `get`, only `get_mut`
    match self.value.get(){
      Some(refcell) => {
        if let Ok(ref_t) = refcell.try_borrow(){
          Some(ref_t)
        } else{
          None
        }
      },
      None => None
    }
  }
 */

}

impl<T> Clone for ValueCell<T> {
  fn clone(&self) -> ValueCell<T> {
    ValueCell{
      value: self.value.clone()
    }
  }
}

impl<T> Default for ValueCell<T>
  where T: Default
{
    fn default() -> Self {
      Self {
        value: Rc::new(RefCell::new(T::default()))
      }
    }
}

impl<T> Hash for ValueCell<T> {
  /// Generate a hash value for the
  /// [`ValueCell<T>`](struct.ValueCell.html).
  ///
  /// This hash value is based on the underlying pointer. Two unique
  /// objects will most likely have different hashes, even if their
  /// values are the same.
  fn hash<H>(&self, state: &mut H) where H: Hasher {
    hash_rc(self.value.clone(), state);
  }
}

impl<T> PartialEq for ValueCell<T> {
  /// Equality for two [`ValueCell<T>`](struct.ValueCell.html)
  /// objects.
  ///
  /// Equality is determined by pointer equality, rather than value
  /// equality. Objects are only considered equal if they point to
  /// the same object.
  fn eq(&self, other: &Self) -> bool {
    Rc::ptr_eq(&self.value, &other.value)
  }
}


impl<T> Deref for ValueCell<T> {
  type Target = RefCell<T>;

  fn deref(&self) -> &Self::Target {
    Rc::deref(&self.value)
  }
}

impl<T> Pointer for ValueCell<T>{
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    self.value.fmt(f)
  }
}
