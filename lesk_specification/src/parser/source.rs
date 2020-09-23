#[cfg(feature = "serialization")]
use serde::{Deserialize, Serialize};
use std::ffi::{OsStr, OsString};
use std::num::{NonZeroU8, NonZeroU32};
use std::{error, fmt};

use codespan::{
  ByteIndex,
  ColumnIndex,
  LineIndex,
  LineIndexOutOfBoundsError,
  LineOffset,
  Location,
  LocationError,
  RawIndex,
  Span,
  SpanOutOfBoundsError,
};

// pub use codespan::FileId;
use codespan_reporting::files::{Files, line_starts};
use std::ops::Range;


/// A file database that contains a single source file.
///
/// Because there is only single file in this database we use `()` as a [`FileId`].
///
/// This is useful for simple language tests, but it might be worth creating a
/// custom implementation when a language scales beyond a certain size.
///
/// [`FileId`]: Files::FileId
#[derive(Debug, Clone)]
pub struct SourceFile<Name, Source> {
  /// The name of the file.
  name: Name,
  /// The source code of the file.
  source: Source,
  /// The starting byte indices in the source code.
  line_starts: Vec<usize>,
}

impl<Name, Source> SourceFile<Name, Source>
  where
      Name: std::fmt::Display,
      Source: AsRef<str>,
{
  /// Create a new source file.
  pub fn new(name: Name, source: Source) -> SourceFile<Name, Source> {
    SourceFile {
      name,
      line_starts: line_starts(source.as_ref()).collect(),
      source,
    }
  }

  /// Return the name of the file.
  pub fn name(&self) -> &Name {
    &self.name
  }

  /// Return the source of the file.
  pub fn source(&self) -> &Source {
    &self.source
  }

  fn line_start(&self, line_index: usize) -> Option<usize> {
    use std::cmp::Ordering;

    match line_index.cmp(&self.line_starts.len()) {
      Ordering::Less => self.line_starts.get(line_index).cloned(),
      Ordering::Equal => Some(self.source.as_ref().len()),
      Ordering::Greater => None,
    }
  }
}

impl<'a, Name, Source> Files<'a> for SourceFile<Name, Source>
  where
      Name: 'a + std::fmt::Display + Clone,
      Source: 'a + AsRef<str>,
{
  type FileId = ();
  type Name = Name;
  type Source = &'a str;

  fn name(&self, (): ()) -> Option<Name> {
    Some(self.name.clone())
  }

  fn source(&self, (): ()) -> Option<&str> {
    Some(self.source.as_ref())
  }

  fn line_index(&self, (): (), byte_index: usize) -> Option<usize> {
    match self.line_starts.binary_search(&byte_index) {
      Ok(line) => Some(line),
      Err(next_line) => Some(next_line - 1),
    }
  }

  fn line_range(&self, (): (), line_index: usize) -> Option<Range<usize>> {
    let line_start = self.line_start(line_index)?;
    let next_line_start = self.line_start(line_index + 1)?;

    Some(line_start..next_line_start)
  }
}

/// A file database that can store multiple source files.
///
/// This is useful for simple language tests, but it might be worth creating a
/// custom implementation when a language scales beyond a certain size.
#[derive(Debug, Clone)]
pub struct SourceFiles<Name, Source> {
  files: Vec<SourceFile<Name, Source>>,
}

impl<Name, Source> SourceFiles<Name, Source>
  where
      Name: std::fmt::Display,
      Source: AsRef<str>,
{
  /// Create a new files database.
  pub fn new() -> SourceFiles<Name, Source> {
    SourceFiles { files: Vec::new() }
  }

  /// Add a file to the database, returning the handle that can be used to
  /// refer to it again.
  pub fn add(&mut self, name: Name, source: Source) -> usize {
    let file_id = self.files.len();
    self.files.push(SourceFile::new(name, source));
    file_id
  }

  /// Get the file corresponding to the given id.
  pub fn get(&self, file_id: usize) -> Option<&SourceFile<Name, Source>> {
    self.files.get(file_id)
  }

  pub fn is_empty(&self) -> bool {
    self.files.is_empty()
  }
}

impl<'a, Name, Source> Files<'a> for SourceFiles<Name, Source>
  where
      Name: 'a + std::fmt::Display + Clone,
      Source: 'a + AsRef<str>,
{
  type FileId = usize;
  type Name = Name;
  type Source = &'a str;

  fn name(&self, file_id: usize) -> Option<Name> {
    Some(self.get(file_id)?.name().clone())
  }

  fn source(&self, file_id: usize) -> Option<&str> {
    Some(self.get(file_id)?.source().as_ref())
  }

  fn line_index(&self, file_id: usize, byte_index: usize) -> Option<usize> {
    self.get(file_id)?.line_index((), byte_index)
  }

  fn line_range(&self, file_id: usize, line_index: usize) -> Option<Range<usize>> {
    self.get(file_id)?.line_range((), line_index)
  }
}
