//! Identifiers are used by lazy_evaluation, tasks, and projects.

use crate::lazy_evaluation::{AnyProp, Prop, VecProp};
use crate::prelude::ProjectResult;
use crate::project::buildable::Buildable;
use crate::project::error::ProjectError;
use crate::task::{BuildableTask, HasTaskId};
use crate::Project;
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{HashSet, VecDeque};
use std::error::Error;
use std::ffi::OsStr;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// The separator between parts of an identifier
pub const ID_SEPARATOR: char = ':';

/// Represents some identifier in an assemble project.
///
/// Acts like a path. Consists for two parts, the `this` part and the `parent`. For example, in
/// `root:inner:task`, the `this` is `task` and the `parent` is `root:inner`.
#[derive(Default, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Id {
    parent: Option<Box<Id>>,
    this: String,
}

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(parent) = self.parent.as_deref() {
            write!(f, "{}{ID_SEPARATOR}{}", parent, self.this)
        } else {
            write!(f, "{ID_SEPARATOR}{}", self.this)
        }
    }
}

impl Debug for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self)
    }
}

impl Id {
    /// Create a new id
    ///
    /// # Error
    /// Errors if it isn't a valid identifier.
    ///
    /// # Example
    /// ```
    /// # use assemble_core::identifier::Id;
    /// let id = Id::new("root:inner:task").unwrap();
    /// assert!(Id::new("&task").is_err());
    /// assert!(Id::new("2132").is_err());
    /// assert!(Id::new("gef::as").is_err());
    /// ```
    pub fn new<S: AsRef<str>>(val: S) -> Result<Self, InvalidId> {
        let as_str = val.as_ref();
        let split = as_str.split(ID_SEPARATOR);
        Self::from_iter(split)
    }

    /// Create a new id that can't be checked
    pub(crate) fn new_uncheckable<S: AsRef<str>>(val: S) -> Self {
        let as_str = val.as_ref();
        let split = as_str.split(ID_SEPARATOR);
        Self::from_iter(split).unwrap()
    }

    /// Try to create an Id from an iterator of parts. Each part must be a valid **part** of an identifier.
    ///
    /// # Example
    /// ```
    /// # use assemble_core::identifier::Id;
    /// assert!(Id::from_iter(["root", "task"]).is_ok());
    /// assert!(Id::from_iter(["root:inner", "task"]).is_err());
    /// assert!(Id::from_iter(["root", "inner", "task"]).is_ok());
    /// ```
    pub fn from_iter<S: AsRef<str>>(iter: impl IntoIterator<Item = S>) -> Result<Self, InvalidId> {
        let mut iterator = iter.into_iter();
        let start = iterator
            .next()
            .ok_or(InvalidId::new(""))
            .and_then(|u| Self::new_unit(u.as_ref()))?;

        iterator.try_fold(start, |accum, obj| {
            let next_id = Self::new_unit(obj.as_ref())?;
            Ok(accum.concat(next_id))
        })
    }

    fn new_unit(id: &str) -> Result<Self, InvalidId> {
        is_valid_identifier(id).map(|_| Id {
            parent: None,
            this: format!("{}", id),
        })
    }

    /// Joins something that can be turned into an identifier to the end of this Id.
    ///
    /// # Error
    /// Errors if the next is not a valid identifier
    pub fn join<S: AsRef<str>>(&self, next: S) -> Result<Self, InvalidId> {
        Id::new(next).map(|id| self.clone().concat(id))
    }

    /// Concatenate two Id's together
    pub fn concat(self, mut other: Self) -> Self {
        other.insert_as_topmost(self);
        other
    }

    fn insert_as_topmost(&mut self, parent: Self) {
        match &mut self.parent {
            Some(p) => p.insert_as_topmost(parent),
            missing => *missing = Some(Box::new(parent)),
        }
    }

    /// Returns this part of an identifier path.
    pub fn this(&self) -> &str {
        &self.this
    }

    /// Returns this part of an identifier path as an [`Id`](Id)
    pub fn this_id(&self) -> Self {
        Id::new_uncheckable(self.this())
    }

    /// Returns the parent identifier of this id, if it exists.
    pub fn parent(&self) -> Option<&Id> {
        self.parent.as_ref().map(|boxed| boxed.as_ref())
    }

    /// Check if the given representation is a valid shorthand.
    pub fn is_shorthand(&self, repr: &str) -> bool {
        let mut shorthands = repr.split(ID_SEPARATOR).rev();
        for ancestor in self.ancestors() {
            if let Some(shorthand) = shorthands.next() {
                if !ancestor.is_shorthand_this(shorthand) {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }

    pub fn is_shorthand_this(&self, repr: &str) -> bool {
        self.this() == repr
    }

    /// Gets the ancestors of this id.
    ///
    /// For example, the ancestors of `root:inner:task` would be
    /// - `root:inner:task`
    /// - `root:inner`
    /// - `root`
    pub fn ancestors(&self) -> impl Iterator<Item = &Id> {
        let mut vec_dequeue = VecDeque::new();
        vec_dequeue.push_front(self);
        let mut ptr = self;
        while let Some(parent) = ptr.parent.as_ref() {
            vec_dequeue.push_back(&*parent);
            ptr = &*parent;
        }

        vec_dequeue.into_iter()
    }

    pub fn iter(&self) -> Iter {
        Iter::new(self)
    }

    pub fn as_path(&self) -> PathBuf {
        PathBuf::from_iter(self.iter())
    }
}

impl From<&str> for Id {
    fn from(id: &str) -> Self {
        Id::new(id).expect("invalid id")
    }
}

impl<S: AsRef<str> + ?Sized> PartialEq<S> for Id {
    fn eq(&self, other: &S) -> bool {
        Id::new(other).map(|id| self == &id).unwrap_or(false)
    }
}

impl PartialEq<str> for &Id {
    fn eq(&self, other: &str) -> bool {
        Id::new(other).map(|id| *self == &id).unwrap_or(false)
    }
}

impl PartialEq<&Id> for Id {
    fn eq(&self, other: &&Id) -> bool {
        self == *other
    }
}

impl PartialEq<Id> for &Id {
    fn eq(&self, other: &Id) -> bool {
        *self == other
    }
}

/// How tasks are referenced throughout projects.
///
/// All tasks **must** have an associated TaskId.
#[derive(Default, Eq, PartialEq, Clone, Hash, Serialize, Deserialize)]
pub struct TaskId(Id);

impl TaskId {
    pub fn new<S: AsRef<str>>(s: S) -> Result<TaskId, InvalidId> {
        Id::new(s).map(Self)
    }

    /// Creates a new empty property. Does not register said property
    pub fn prop<T: Clone + Send + Sync + 'static>(&self, name: &str) -> Result<Prop<T>, InvalidId> {
        let id = self.join(name)?;
        Ok(Prop::new(id))
    }

    /// Creates a new vec property. Does not register said property
    pub fn vec_prop<T: Clone + Send + Sync + 'static>(
        &self,
        name: &str,
    ) -> Result<VecProp<T>, InvalidId> {
        let id = self.join(name)?;
        Ok(VecProp::new(id))
    }
}

impl Debug for TaskId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl Buildable for TaskId {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        // println!("Attempting to get dependencies for {} in {}", self, project);
        // let info = project
        //     .task_container()
        //     .get_task(self)?;
        // println!("got info: {:#?}", info.task_id());
        let mut output: HashSet<TaskId> = HashSet::new();
        output.insert(self.clone());
        Ok(output)
    }
}

impl Buildable for &str {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        let task_id = project.find_task_id(self)?;
        task_id.get_dependencies(project)
    }
}

impl Deref for TaskId {
    type Target = Id;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<TaskId> for TaskId {
    fn as_ref(&self) -> &TaskId {
        self
    }
}

impl From<&TaskId> for TaskId {
    fn from(t: &TaskId) -> Self {
        t.clone()
    }
}

impl TryFrom<&str> for TaskId {
    type Error = InvalidId;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for TaskId {
    type Error = InvalidId;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Id> for TaskId {
    fn from(i: Id) -> Self {
        Self(i)
    }
}

/// How projects are referenced. Unlike tasks, projects don't have to have parents.
#[derive(Default, Eq, PartialEq, Clone, Hash, Serialize, Deserialize)]
pub struct ProjectId(Id);

impl ProjectId {
    pub fn root() -> Self {
        Self(Id::new("root").unwrap())
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, InvalidId> {
        let mut path = path.as_ref();
        if let Ok(prefixless) = path.strip_prefix("/") {
            path = prefixless;
        }
        let iter = path
            .into_iter()
            .map(|s| {
                s.to_str()
                    .ok_or(InvalidId::new(path.to_string_lossy().to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Id::from_iter(iter).map(Self)
    }

    pub fn new(id: &str) -> Result<Self, InvalidId> {
        let name = Id::new(id)?;
        Ok(ProjectId(name))
    }
}

impl Debug for ProjectId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl TryFrom<&Path> for ProjectId {
    type Error = InvalidId;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        Self::from_path(value)
    }
}

impl TryFrom<&str> for ProjectId {
    type Error = InvalidId;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Id> for ProjectId {
    fn from(id: Id) -> Self {
        Self(id)
    }
}

impl Deref for ProjectId {
    type Target = Id;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

macro_rules! deref_to_id {
    ($ty:ty) => {
        impl<I: ?Sized> PartialEq<I> for $ty
        where
            Id: PartialEq<I>,
        {
            fn eq(&self, other: &I) -> bool {
                self.deref().eq(other)
            }
        }

        impl Display for $ty {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.deref())
            }
        }

        impl FromStr for $ty {
            type Err = InvalidId;

            /// Parses a task ID. Unlike the TryFrom methods, this one can produced multi level ids
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let mut output: Option<Id> = None;
                if !s.starts_with(':') {
                    return Err(InvalidId::new(s));
                }

                for task_part in s[1..].split(":") {
                    match output {
                        Some(old_output) => output = Some(old_output.join(task_part)?),
                        None => output = Some(Id::new(task_part)?),
                    }
                }
                output
                    .map(|id| <$ty>::from(id))
                    .ok_or(InvalidId(s.to_string()))
            }
        }
    };
}

deref_to_id!(TaskId);
deref_to_id!(ProjectId);

/// Create new tasks Ids
#[derive(Clone, Debug)]
pub struct TaskIdFactory {
    project: ProjectId,
}

impl TaskIdFactory {
    pub(crate) fn new(project: ProjectId) -> Self {
        Self { project }
    }

    pub fn create(&self, task_name: impl AsRef<str>) -> Result<TaskId, InvalidId> {
        self.project.join(task_name).map(TaskId)
    }
}

#[derive(Debug)]
pub struct InvalidId(pub String);

impl InvalidId {
    pub fn new(string: impl AsRef<str>) -> Self {
        Self(string.as_ref().to_string())
    }
}

impl Display for InvalidId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid Task Identifier {:?}", self.0)
    }
}

impl Error for InvalidId {}

pub fn is_valid_identifier(id: &str) -> Result<(), InvalidId> {
    static VALID_ID_PATTERN: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"[a-zA-Z][\w-]*").expect("Invalid Pattern"));

    VALID_ID_PATTERN
        .find(id)
        .ok_or(InvalidId::new(id))
        .and_then(|mat| {
            if mat.as_str() == id {
                Ok(())
            } else {
                Err(InvalidId::new(id))
            }
        })
}

pub struct Iter<'id> {
    ids: Vec<&'id Id>,
}

impl<'i> Iter<'i> {
    fn new(id: &'i Id) -> Self {
        let ancestors = id.ancestors();
        let vec = Vec::from_iter(ancestors);
        Self { ids: vec }
    }
}

impl<'i> Iterator for Iter<'i> {
    type Item = &'i str;

    fn next(&mut self) -> Option<Self::Item> {
        let top = self.ids.pop()?;
        Some(top.this())
    }
}

#[cfg(test)]
mod tests {
    use crate::identifier::Id;

    #[test]
    fn from_string() {
        let id = Id::from_iter(&["project", "task"]).unwrap();
        let other_id = Id::new("project:task");
    }

    #[test]
    fn to_string() {
        let id = Id::from_iter(&["project", "task"]).unwrap();
        assert_eq!(id.to_string(), ":project:task");

        let id = Id::from_iter(&["task"]).unwrap();
        assert_eq!(id.to_string(), ":task");
    }

    #[test]
    fn ancestors() {
        let id = Id::new_uncheckable("root:child:task");
        let mut ancestors = id.ancestors();
        assert_eq!(
            ancestors.next(),
            Some("root:child:task").map(Id::new_uncheckable).as_ref()
        );
        assert_eq!(
            ancestors.next(),
            Some("root:child").map(Id::new_uncheckable).as_ref()
        );
        assert_eq!(
            ancestors.next(),
            Some("root").map(Id::new_uncheckable).as_ref()
        );
        assert_eq!(ancestors.next(), None);
    }

    #[test]
    fn is_shorthand() {
        let id = Id::from_iter(&["project", "task"]).unwrap();
        let shorthand = "project:task";
        assert!(id.is_shorthand(shorthand));
    }
}
