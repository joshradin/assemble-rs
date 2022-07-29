/// Defines types of file collections and the FileCollection trait
use std::collections::HashSet;
use std::env::{join_paths, JoinPathsError};
use std::ffi::OsString;
use std::fmt::{Debug, Formatter};
use std::fs::DirEntry;
use std::ops::{Add, AddAssign, Not};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

use crate::file::RegularFile;
use crate::identifier::TaskId;
use crate::project::buildable::{Buildable, BuiltByContainer, IntoBuildable};
use crate::project::ProjectError;
use crate::utilities::{AndSpec, Spec, True};
use crate::Project;
use itertools::Itertools;
use crate::file_collection::fileset::FileSet;

pub mod configuration;
pub mod fileset;

/// A collection of files.
pub trait FileCollection : Send + Sync + Buildable {
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

/// An iterator over file components.
pub struct FileIterator<'files> {
    components: &'files [Component],
    filters: &'files dyn FileFilter,
    index: usize,
    current_iterator: Option<Box<dyn Iterator<Item = PathBuf> + 'files>>,
}

impl<'files> FileIterator<'files> {
    fn next_iterator(&mut self) -> Option<Box<dyn Iterator<Item = PathBuf> + 'files>> {
        if self.index == self.components.len() {
            return None;
        }

        let output = Some(self.components[self.index].iter());
        self.index += 1;
        output
    }

    fn get_next_path(&mut self) -> Option<PathBuf> {
        'OUTER: loop {
            if self.current_iterator.is_none() {
                self.current_iterator = self.next_iterator();
            }

            if let Some(iterator) = &mut self.current_iterator {
                while let Some(path) = iterator.next() {
                    if self.filters.accept(&path) {
                        break 'OUTER Some(path);
                    }
                }
            } else {
                break None;
            }
        }
    }
}

impl<'files> Iterator for FileIterator<'files> {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        self.get_next_path()
    }
}


pub trait FileFilter: Spec<Path> + Send + Sync {}

assert_obj_safe!(FileFilter);

impl<F> FileFilter for F where F: Spec<Path> + Send + Sync {}

impl Spec<Path> for glob::Pattern {
    fn accept(&self, value: &Path) -> bool {
        self.matches_path(value)
    }
}