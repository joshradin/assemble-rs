//! The file set is just a set of files

use crate::file_collection::{Component, FileCollection, FileFilter, FileIterator};
use itertools::Itertools;
use std::collections::HashSet;
use std::env::{join_paths, JoinPathsError};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use std::fmt::{Debug, Formatter};
use std::ops::{Add, AddAssign};
use std::sync::Arc;
use crate::__export::TaskId;
use crate::Project;
use crate::project::buildable::{Buildable, BuiltByContainer, IntoBuildable};
use crate::project::ProjectError;
use crate::utilities::{AndSpec, True};


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
pub struct FileSet {
    filter: Arc<dyn FileFilter>,
    built_by: BuiltByContainer,
    components: Vec<Component>,
}

impl FileSet {
    pub fn new() -> Self {
        Self {
            filter: Arc::new(True::new()),
            built_by: BuiltByContainer::default(),
            components: vec![],
        }
    }

    pub fn with_path(path: impl AsRef<Path>) -> Self {
        Self {
            filter: Arc::new(True::new()),
            built_by: BuiltByContainer::default(),
            components: vec![Component::Path(path.as_ref().to_path_buf())],
        }
    }

    pub fn built_by<B: IntoBuildable>(&mut self, b: B)
    where
        <B as IntoBuildable>::Buildable: 'static,
    {
        self.built_by.add(b);
    }

    pub fn join(self, other: Self) -> Self {
        Self {
            components: vec![Component::Collection(self), Component::Collection(other)],
            ..Default::default()
        }
    }

    pub fn iter(&self) -> FileIterator {
        self.into_iter()
    }

    pub fn filter<F: FileFilter + 'static>(&mut self, filter: F) {
        let prev = std::mem::replace(&mut self.filter, Arc::new(True::new()));
        let and = AndSpec::new(prev, filter);
        self.filter = Arc::new(and);
    }
}

impl Default for FileSet {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for FileSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "FileCollection {{ ... }}")
    }
}

impl<F: Into<FileSet>> Add<F> for FileSet {
    type Output = Self;

    fn add(self, rhs: F) -> Self::Output {
        self.join(rhs.into())
    }
}

impl<F: Into<FileSet>> AddAssign<F> for FileSet {
    fn add_assign(&mut self, rhs: F) {
        let old = std::mem::replace(self, FileSet::default());
        *self = old.join(rhs.into())
    }
}

impl<P: AsRef<Path>> From<P> for FileSet {
    fn from(path: P) -> Self {
        Self::with_path(path)
    }
}

impl Buildable for FileSet {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        self.built_by.get_dependencies(project)
    }
}
