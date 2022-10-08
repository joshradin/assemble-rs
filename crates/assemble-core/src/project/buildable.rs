//! Define the buildable trait
//!
//! Buildable traits allows for easier constructing of project structures.
//!
//! # Buildable Types
//! - `String`, `&str`, or [`TaskId`](crate::__export::TaskId)
//! - Any type that implements [`TaskDependency`](TaskDependency)
//! - Any type that implements [`Buildable`](Buildable)
//! - [`FileCollection`](crate::file_collection::FileCollection)

use crate::identifier::{Id, TaskId};
use crate::project::error::ProjectError;
use crate::project::ProjectResult;
use crate::task::Executable;
use crate::{project::Project, Task};
use itertools::Itertools;
use log::{debug, info};
use std::any::{type_name, Any};
use std::borrow::Borrow;
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// Represents something can be _built_ by the assemble project.
pub trait IntoBuildable {
    type Buildable: Buildable;
    /// Returns a dependency which contains the tasks which build this object.
    fn into_buildable(self) -> Self::Buildable;
}

pub trait GetBuildable {
    /// Returns a dependency which contains the tasks which build this object.
    fn as_buildable(&self) -> BuildableObject;
}

assert_obj_safe!(GetBuildable);

impl<B: IntoBuildable + Clone> GetBuildable for B
where
    <B as IntoBuildable>::Buildable: 'static,
{
    fn as_buildable(&self) -> BuildableObject {
        BuildableObject::new(self.clone().into_buildable())
    }
}

impl<B: Buildable> IntoBuildable for B {
    type Buildable = B;

    fn into_buildable(self) -> B {
        self
    }
}

/// The tasks that are required to be built by this project to make this object. If this is a task,
/// the task is also included.
pub trait Buildable: Send + Sync + Debug {
    /// Gets the dependencies required to build this task
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>>;
}

assert_obj_safe!(Buildable);

impl<B: Buildable> Buildable for &B {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        (*self).get_dependencies(project)
    }
}

impl Buildable for Box<dyn Buildable + '_> {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        self.as_ref().get_dependencies(project)
    }
}

impl Buildable for Arc<dyn Buildable + '_> {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        self.as_ref().get_dependencies(project)
    }
}

impl<B: Buildable> Buildable for Vec<B> {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        self.into_iter()
            .map(|b| b.get_dependencies(project))
            .collect::<Result<Vec<HashSet<_>>, _>>()
            .map(|v| v.into_iter().flatten().collect())
    }
}

/// A set of task dependencies
#[derive(Default, Clone)]
pub struct BuiltByContainer(Vec<Arc<dyn Buildable>>);

impl BuiltByContainer {
    /// Creates a new, empty built by container
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Creates a new, empty built by container with a preset capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// Creates a built by container that already contains a given buildable
    pub fn with_buildable<B: IntoBuildable>(buildable: B) -> Self
    where
        <B as IntoBuildable>::Buildable: 'static,
    {
        let mut output = BuiltByContainer::with_capacity(1);
        output.add(buildable);
        output
    }

    /// Join two BuiltByContainers together
    pub fn join(self, other: Self) -> Self {
        let mut inner = self.0;
        inner.extend(other.0);
        Self(inner)
    }

    pub fn add<T: IntoBuildable>(&mut self, buildable: T)
    where
        <T as IntoBuildable>::Buildable: 'static,
    {
        let buildable: Arc<dyn Buildable> = Arc::new(buildable.into_buildable());
        self.0.push(buildable);
    }
}

impl Debug for BuiltByContainer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "BuiltByContainer ")?;
        f.debug_set().entries(&self.0).finish()
    }
}

impl Buildable for BuiltByContainer {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        let mut output = HashSet::new();
        for dep in &self.0 {
            trace!("Getting dependencies for buildable: {:#?}", dep);
            output.extend(dep.get_dependencies(project)?);
        }
        Ok(output)
    }
}

/// Allows for adding "built by" info to non buildable objects
#[derive(Debug)]
pub struct BuiltBy<T: Debug> {
    built_by: Arc<dyn Buildable>,
    value: T,
}

impl<T: Debug + Clone> Clone for BuiltBy<T> {
    fn clone(&self) -> Self {
        Self {
            built_by: self.built_by.clone(),
            value: self.value.clone(),
        }
    }
}

impl<T: Debug> DerefMut for BuiltBy<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: Debug> Deref for BuiltBy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Debug> IntoBuildable for BuiltBy<T> {
    type Buildable = Arc<dyn Buildable>;

    fn into_buildable(self) -> Self::Buildable {
        self.built_by
    }
}

impl<T: Debug> BuiltBy<T> {
    /// Create a new buildable object
    pub fn new<B: IntoBuildable + 'static>(built_by: B, value: T) -> Self {
        Self {
            built_by: Arc::new(built_by.into_buildable()),
            value,
        }
    }

    /// Makes this into the inner value
    pub fn into_inner(self) -> T {
        self.value
    }

    /// Turns this built by into a built of a reference
    pub fn as_ref(&self) -> BuiltBy<&T> {
        BuiltBy {
            built_by: self.built_by.clone(),
            value: &self.value,
        }
    }

    /// Gets a copy of the buildable
    pub fn built_by(&self) -> &dyn Buildable {
        &self.built_by
    }
}

/// Holds various type of Buildable
#[derive(Clone, Debug)]
pub enum BuildableObject {
    /// Wrap a container
    Container(BuiltByContainer),
    /// Wrap a task id
    Id(TaskId),
    /// Wrap any other type
    Other(Arc<dyn Buildable>),
    /// Represents a buildable with no task dependencies
    None,
}

impl BuildableObject {
    /// Create a buildable object from something that can be turned into a buildable
    pub fn new<B: IntoBuildable>(buildable: B) -> Self
    where
        <B as IntoBuildable>::Buildable: 'static,
    {
        Self::Other(Arc::new(buildable.into_buildable()))
    }
}

impl Buildable for BuildableObject {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        match self {
            BuildableObject::Container(c) => c.get_dependencies(project),
            BuildableObject::Id(id) => id.get_dependencies(project),
            BuildableObject::Other(o) => o.get_dependencies(project),
            BuildableObject::None => Ok(HashSet::new()),
        }
    }
}

impl From<BuiltByContainer> for BuildableObject {
    fn from(c: BuiltByContainer) -> Self {
        BuildableObject::Container(c)
    }
}

impl From<TaskId> for BuildableObject {
    fn from(c: TaskId) -> Self {
        BuildableObject::Id(c)
    }
}

impl From<Box<dyn Buildable>> for BuildableObject {
    fn from(boxed: Box<dyn Buildable>) -> Self {
        let arc = Arc::from(boxed);
        BuildableObject::Other(arc)
    }
}

assert_impl_all!(BuildableObject: Buildable, IntoBuildable, GetBuildable);
