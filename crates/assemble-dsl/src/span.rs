//! A span with a program.

use std::ops::{Range, RangeFrom};
use std::path::{Path, PathBuf};

/// A span used to track where tokens originate from.
///
/// Represents a file (if available) and a character range (if available)
#[derive(Debug, PartialEq, Clone)]
pub struct Span {
    path: Option<PathBuf>,
    range: Range<usize>,
}

impl Span {
    /// Gets the range of the span
    pub fn range(&self) -> &Range<usize> {
        &self.range
    }

    /// Gets the path of the span
    pub fn path(&self) -> Option<&Path> {
        self.path.as_ref().map(|p| p.as_path())
    }
}
