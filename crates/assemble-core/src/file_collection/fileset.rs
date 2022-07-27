//! The file set is just a set of files

use crate::file_collection::{FileIterator, FileSet};
use itertools::Itertools;
use std::collections::HashSet;
use std::env::{join_paths, JoinPathsError};
use std::ffi::OsString;
use std::path::PathBuf;
use walkdir::WalkDir;

/// A collection of files.
pub trait FileCollection {
    /// Gets a set of files that make up this file collection
    fn files(&self) -> HashSet<PathBuf>;

    /// Gets whether this file collection contains any files
    fn is_empty(&self) -> bool {
        self.files().is_empty()
    }

    /// Create a PATH based on the files in this collection
    fn as_path(&self) -> Result<OsString, JoinPathsError> {
        join_paths(self.files())
    }
}

impl FileCollection for FileSet {
    fn files(&self) -> HashSet<PathBuf> {
        HashSet::from_iter(self.iter())
    }
}

impl<'f> IntoIterator for &'f FileSet {
    type Item = PathBuf;
    type IntoIter = FileIterator<'f>;

    fn into_iter(self) -> Self::IntoIter {
        FileIterator {
            components: &self.components[..],
            filters: &*self.filter,
            index: 0,
            current_iterator: None,
        }
    }
}

#[derive(Clone)]
pub enum Component {
    Path(PathBuf),
    Collection(FileSet),
}

impl Component {
    pub fn iter(&self) -> Box<dyn Iterator<Item = PathBuf> + '_> {
        match self {
            Component::Path(p) => {
                if p.is_file() {
                    Box::new(Some(p.clone()).into_iter())
                } else {
                    Box::new(
                        WalkDir::new(p)
                            .into_iter()
                            .map_ok(|entry| entry.into_path())
                            .map(|res| res.unwrap().to_path_buf()),
                    )
                }
            }
            Component::Collection(c) => Box::new(c.iter()),
        }
    }
}

impl<'f> IntoIterator for &'f Component {
    type Item = PathBuf;
    type IntoIter = Box<dyn Iterator<Item = PathBuf> + 'f>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
