use std::fmt::{Debug, Formatter};
use std::fs::DirEntry;
use std::ops::{Add, AddAssign, Not};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::file::RegularFile;
use crate::utilities::{AndSpec, Spec, True};
use itertools::Itertools;
use crate::__export::TaskId;
use crate::project::buildable::Buildable;

pub struct FileCollection {
    filter: Box<dyn FileFilter>,
    build_by: Vec<Box<dyn Buildable>>,
    components: Vec<Component>,
}

impl FileCollection {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            filter: Box::new(True::new()),
            build_by: vec![],
            components: vec![Component::Path(path.as_ref().to_path_buf())],
        }
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
        let prev = std::mem::replace(&mut self.filter, Box::new(True::new()));
        let and = AndSpec::new(prev, filter);
        self.filter = Box::new(and);
    }
}

impl Default for FileCollection {
    fn default() -> Self {
        Self {
            filter: Box::new(True::new()),
            components: vec![],
        }
    }
}

impl<'f> IntoIterator for &'f FileCollection {
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

impl Debug for FileCollection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "FileCollection {{ ... }}")
    }
}

impl<F: Into<FileCollection>> Add<F> for FileCollection {
    type Output = Self;

    fn add(self, rhs: F) -> Self::Output {
        self.join(rhs.into())
    }
}

impl<F: Into<FileCollection>> AddAssign<F> for FileCollection {
    fn add_assign(&mut self, rhs: F) {
        let old = std::mem::replace(self, FileCollection::default());
        *self = old.join(rhs.into())
    }
}

impl<P: AsRef<Path>> From<P> for FileCollection {
    fn from(path: P) -> Self {
        Self::new(path)
    }
}

pub enum Component {
    Path(PathBuf),
    Collection(FileCollection),
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

pub trait FileFilter: Spec<Path> {}

assert_obj_safe!(FileFilter);

impl<F> FileFilter for F where F: Spec<Path> {}


impl Spec<Path> for glob::Pattern {
    fn accept(&self, value: &Path) -> bool {
        self.matches_path(value)
    }
}
