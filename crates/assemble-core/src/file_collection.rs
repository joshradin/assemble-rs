/// Defines types of file collections and the FileCollection trait
use std::collections::{HashSet, LinkedList, VecDeque};
use std::convert::identity;
use std::env::JoinPathsError;
use std::ffi::OsString;
use std::fmt::{Debug, Formatter};
use std::fs::DirEntry;
use std::iter::FusedIterator;
use std::ops::{Add, AddAssign, Not};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use itertools::Itertools;
use walkdir::WalkDir;

use crate::exception::{BuildError, BuildException};
use crate::file::RegularFile;
use crate::identifier::TaskId;
use crate::lazy_evaluation::ProviderExt;
use crate::lazy_evaluation::{IntoProvider, Prop, Provider};
use crate::project::buildable::{Buildable, BuiltByContainer, IntoBuildable};
use crate::project::error::ProjectError;
use crate::project::ProjectResult;
use crate::utilities::{AndSpec, Callback, Spec, True};
use crate::{BuildResult, Project};

/// A file set is a collection of files. File collections are intended to be live.
pub trait FileCollection {
    /// Gets the files contained by this collection.
    fn files(&self) -> HashSet<PathBuf>;
    /// Gets the files contained by this collection. Is fallible.
    fn try_files(&self) -> BuildResult<HashSet<PathBuf>> {
        Ok(self.files())
    }
    /// Gets whether this file collection is empty or not
    fn is_empty(&self) -> bool {
        self.files().is_empty()
    }
    /// Get this file collection as a path
    fn path(&self) -> Result<OsString, JoinPathsError> {
        std::env::join_paths(self.files())
    }
}

macro_rules! implement_file_collection {
    ($ty:tt) => {
        impl<P: AsRef<Path>> FileCollection for $ty<P> {
            fn files(&self) -> HashSet<PathBuf> {
                self.iter().map(|p| p.as_ref().to_path_buf()).collect()
            }
        }
    };
}
implement_file_collection!(HashSet);
implement_file_collection!(Vec);
implement_file_collection!(VecDeque);
implement_file_collection!(LinkedList);
// impl<P : AsRef<Path>> FileCollection for HashSet<P> {
//     fn files(&self) -> HashSet<PathBuf> {
//         self.iter()
//             .map(|p| p.as_ref().to_path_buf())
//             .collect()
//     }
// }
//
// impl<P : AsRef<Path>> FileCollection for Vec<P> {
//     fn files(&self) -> HashSet<PathBuf> {
//         self.iter()
//             .map(|p| p.as_ref().to_path_buf())
//             .collect()
//     }
// }

impl FileCollection for PathBuf {
    fn files(&self) -> HashSet<PathBuf> {
        HashSet::from_iter([self.clone()])
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
        if f.alternate() {
            let files = self.files();
            f.debug_set().entries(files).finish()
        } else {
            f.debug_struct("FileSet")
             .field("components", &self.components)
             .finish_non_exhaustive()
        }
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

    pub fn with_path_providers<I, P>(providers: I) -> Self
    where
        I: IntoIterator<Item = P>,
        P: IntoProvider<PathBuf>,
        <P as IntoProvider<PathBuf>>::Provider: 'static,
    {
        let mut output = Self::new();
        for provider in providers {
            output += FileSet::with_provider(provider);
        }
        output
    }

    pub fn with_provider<F: FileCollection, P: IntoProvider<F>>(fc_provider: P) -> Self
    where
        F: Send + Sync + Clone + 'static,
        P::Provider: 'static,
    {
        let mut prop: Prop<FileSet> = Prop::default();
        let provider = fc_provider.into_provider();
        prop.set_with(provider.map(|f: F| FileSet::from_iter(f.files())))
            .unwrap();
        let component = Component::Provider(prop);
        Self {
            filter: Arc::new(True::new()),
            built_by: BuiltByContainer::default(),
            components: vec![component],
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

    /// Adds a filter to a fileset
    pub fn filter<F: FileFilter + 'static>(self, filter: F) -> Self {
        let mut files = self;
        let prev = std::mem::replace(&mut files.filter, Arc::new(True::new()));
        let and = AndSpec::new(prev, filter);
        files.filter = Arc::new(and);
        files
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

    fn try_files(&self) -> BuildResult<HashSet<PathBuf>> {
        Ok(self
            .components
            .iter()
            .map(|c| c.try_files())
            .collect::<Result<Vec<HashSet<_>>, _>>()?
            .into_iter()
            .flatten()
            .filter(|p| self.filter.accept(&*p))
            .collect())
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
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
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

impl Provider<FileSet> for FileSet {
    fn try_get(&self) -> Option<FileSet> {
        Some(self.clone())
    }
}

#[derive(Clone)]
pub enum Component {
    Path(PathBuf),
    Collection(FileSet),
    Provider(Prop<FileSet>),
}

impl Debug for Component {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            match self {
                Component::Path(p) => {
                    write!(f, "{:?}", p)
                }
                Component::Collection(c) => {
                    write!(f, "{:#?}", c)
                }
                Component::Provider(p) => {
                    write!(f, "{:#?}", p)
                }
            }
        } else {
            match self {
                Component::Path(p) => {
                    f.debug_tuple("Path").field(p).finish()
                }
                Component::Collection(c) => {
                    f.debug_tuple("Collection").field(c).finish()
                }
                Component::Provider(p) => {
                    f.debug_tuple("Provider").field(p).finish()
                }
            }
        }
    }
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

impl FileCollection for Component {
    fn files(&self) -> HashSet<PathBuf> {
        self.iter().collect()
    }

    fn try_files(&self) -> BuildResult<HashSet<PathBuf>> {
        Ok(match self {
            Component::Path(p) => {
                if p.is_file() || !p.exists() {
                    Box::new(Some(p.clone()).into_iter()) as Box<dyn Iterator<Item = PathBuf> + '_>
                } else {
                    Box::new(
                        WalkDir::new(p)
                            .into_iter()
                            .map_ok(|entry| entry.into_path())
                            .map(|r| r.map_err(|e| BuildException::new(e)))
                            .collect::<Result<HashSet<PathBuf>, _>>()?
                            .into_iter(),
                    ) as Box<dyn Iterator<Item = PathBuf> + '_>
                }
            }
            Component::Collection(c) => {
                Box::new(c.iter()) as Box<dyn Iterator<Item = PathBuf> + '_>
            }
            Component::Provider(pro) => {
                let component = pro.fallible_get()?;
                Box::new(component.into_iter()) as Box<dyn Iterator<Item = PathBuf> + '_>
            }
        }
        .collect())
    }
}

// impl Debug for dyn Provider<Component> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(f, "Component Provider")
//     }
// }

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

impl<F> FileFilter for F where F: Spec<Path> + Send + Sync + ?Sized {}

impl Spec<Path> for glob::Pattern {
    fn accept(&self, value: &Path) -> bool {
        self.matches_path(value)
    }
}

impl Spec<Path> for &str {
    fn accept(&self, value: &Path) -> bool {
        glob::Pattern::from_str(self).unwrap().accept(value)
    }
}
