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
use crate::project::ProjectError;
use crate::{project::Project, DefaultTask, Executable, Task};
use itertools::Itertools;
use std::any::type_name;
use std::borrow::Borrow;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// Represents something can be _built_ by the assemble project.
pub trait IntoBuildable {
    type Buildable: Buildable;

    /// Returns a dependency which contains the tasks which build this object.
    fn into_buildable(self) -> Self::Buildable;
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
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError>;
}

assert_obj_safe!(Buildable);

impl<B: Buildable> Buildable for &B {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        (*self).get_dependencies(project)
    }
}

impl Buildable for Box<dyn Buildable + '_> {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        self.as_ref().get_dependencies(project)
    }
}

impl Buildable for Arc<dyn Buildable + '_> {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        self.as_ref().get_dependencies(project)
    }
}

/// A set of task dependencies
#[derive(Default, Clone)]
pub struct BuildByContainer(Vec<Arc<dyn Buildable>>);

impl BuildByContainer {
    pub const fn new() -> Self {
        Self(Vec::new())
    }
}

impl Debug for BuildByContainer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(type_name::<Self>()).finish_non_exhaustive()
    }
}

impl BuildByContainer {
    pub fn add<T: IntoBuildable>(&mut self, buildable: T)
    where
        <T as IntoBuildable>::Buildable: 'static,
    {
        let buildable: Arc<dyn Buildable> = Arc::new(buildable.into_buildable());
        self.0.push(buildable);
    }
}

impl Buildable for BuildByContainer {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        let mut output = HashSet::new();
        for dep in &self.0 {
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
