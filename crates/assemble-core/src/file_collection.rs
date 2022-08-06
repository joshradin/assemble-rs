use std::collections::HashSet;
use std::env::JoinPathsError;
use std::ffi::OsString;
use std::fmt::{Debug, Formatter};
use std::fs::DirEntry;
use std::iter::FusedIterator;
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
use crate::properties::{Prop, Provides};

/// A file set is a collection of files. File collections are intended to be live.
pub trait FileCollection {
    /// Gets the files contained by this collection.
    fn files(&self) -> HashSet<PathBuf>;
    /// Gets whether this file collection is empty or not
    fn is_empty(&self) -> bool {
        self.files().is_empty()
    }
    /// Get this file collection as a path
    fn path(&self) -> Result<OsString, JoinPathsError> {
        std::env::join_paths(self.files())
    }
}

#[derive(Clone)]
pub struct FileSet {
    filter: Arc<dyn FileFilter>,
    built_by: BuiltByContainer,
    components: Vec<Component>,
}

impl Debug for FileSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileSet")
            .field("components", &self.components)
            .finish_non_exhaustive()
    }
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

    pub fn insert<T: Into<FileSet>>(&mut self, fileset: T) {
        *self += fileset;
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

impl<'f> IntoIterator for FileSet {
    type Item = PathBuf;
    type IntoIter = std::vec::IntoIter<PathBuf>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter().collect::<Vec<_>>().into_iter()
    }
}

impl FileCollection for FileSet {
    fn files(&self) -> HashSet<PathBuf> {
        self.iter().collect()
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

impl<P: AsRef<Path>> FromIterator<P> for FileSet {
    fn from_iter<T: IntoIterator<Item = P>>(iter: T) -> Self {
        iter.into_iter()
            .map(|p: P| FileSet::with_path(p))
            .reduce(|accum, next| accum + next)
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug)]
pub enum Component {
    Path(PathBuf),
    Collection(FileSet),
    Provider(Prop<FileSet>)
}

impl Component {
    pub fn iter(&self) -> Box<dyn Iterator<Item = PathBuf> + '_> {
        match self {
            Component::Path(p) => {
                if p.is_file() || !p.exists() {
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
            Component::Provider(pro) => {
                let component = pro.get();
                Box::new(component.into_iter())
            }
        }
    }
}

impl Debug for dyn Provides<Component> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Component Provider")
    }
}

impl<'f> IntoIterator for &'f Component {
    type Item = PathBuf;
    type IntoIter = Box<dyn Iterator<Item = PathBuf> + 'f>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct FileIterator<'files> {
    components: &'files [Component],
    filters: &'files dyn FileFilter,
    index: usize,
    current_iterator: Option<Box<dyn Iterator<Item = PathBuf> + 'files>>,
}

impl<'files> Debug for FileIterator<'files> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileIterator")
            .field("components", &self.components)
            .field("index", &self.index)
            .finish_non_exhaustive()
    }
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
        if self.index == self.components.len() {
            return None;
        }
        loop {
            if self.current_iterator.is_none() {
                self.current_iterator = self.next_iterator();
            }

            if let Some(iterator) = &mut self.current_iterator {
                while let Some(path) = iterator.next() {
                    if self.filters.accept(&path) {
                        return Some(path);
                    }
                }
                self.current_iterator = None;
            } else {
                return None;
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

impl<'files> FusedIterator for FileIterator<'files> {}

pub trait FileFilter: Spec<Path> + Send + Sync {}

assert_obj_safe!(FileFilter);

impl<F> FileFilter for F where F: Spec<Path> + Send + Sync {}

impl Spec<Path> for glob::Pattern {
    fn accept(&self, value: &Path) -> bool {
        self.matches_path(value)
    }
}

