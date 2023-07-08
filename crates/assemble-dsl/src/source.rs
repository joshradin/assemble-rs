//! Source files

use crate::span::Span;
use std::cell::{Cell, RefCell};
use std::fs::File;
use std::io::Read;
use std::ops::{Range, RangeBounds};
use std::path::{Path, PathBuf};
use std::string::FromUtf8Error;
use std::{fs, io};

/// A source file.
pub struct Source {
    kind: SourceKind,
}

impl Source {
    /// Creates a new source value in a file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, SourceError> {
        Ok(Self {
            kind: SourceKind::File {
                path: path.as_ref().to_path_buf(),
                cache: RefCell::new(None),
            },
        })
    }

    /// Creates a new source entity from a string
    pub fn from_raw(raw: impl AsRef<str>) -> Self {
        Self {
            kind: SourceKind::Raw(raw.as_ref().to_string()),
        }
    }

    pub fn read(&self, span: &Span) -> &str {
        self.try_read(span).expect("could not read span")
    }
    /// Tries to read a span
    pub fn try_read(&self, span: &Span) -> Result<&str, SourceError> {
        let range = span.range();
        self._text_at_range(range)
    }

    fn _text_at_range(&self, range: &Range<usize>) -> Result<&str, SourceError> {
        let str = self.text()?;
        Self::read_string(range, str)
    }

    /// Gets the text within the source file
    pub fn text(&self) -> Result<&str, SourceError> {
        Ok(match &self.kind {
            SourceKind::Raw(raw) => &*raw,
            SourceKind::File { path, cache } => {
                if cache.borrow().is_none() {
                    let mut cache = cache.borrow_mut();
                    let read = String::from_utf8(fs::read(path)?)?;
                    let _ = cache.insert(read);
                }

                let borrowed = cache.borrow();
                let str = borrowed.as_ref().unwrap();
                &*str
            }
        })
    }

    fn read_string(range: &Range<usize>, raw: &str) -> Result<&str, SourceError> {
        if range.start >= raw.len() {
            Err(SourceError::OutOfBounds {
                index: range.start,
                length: raw.len(),
            })
        } else if range.end >= raw.len() {
            Err(SourceError::OutOfBounds {
                index: range.end,
                length: raw.len(),
            })
        } else {
            let s = &raw[range.clone()];
            Ok(s)
        }
    }
}

enum SourceKind {
    Raw(String),
    File {
        path: PathBuf,
        cache: RefCell<Option<String>>,
    },
}

/// An error retrieving the actual source
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("Index out of bounds (index: {}, length: {})", index, length)]
    OutOfBounds { index: usize, length: usize },
    #[error("Source mismatch (expected: {:?}, found: {:?})", expected, found)]
    SourceMismatch {
        expected: Option<PathBuf>,
        found: Option<PathBuf>,
    },
    #[error(transparent)]
    IOError(#[from] io::Error),
    #[error(transparent)]
    FromUTf8Error(#[from] FromUtf8Error),
}
