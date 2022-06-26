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
use std::collections::HashSet;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

/// Represents something can be _built_ by the assemble project.
pub trait Buildable: Send + Sync + Debug {
    /// Returns a dependency which contains the tasks which build this object.
    fn get_build_dependencies(&self) -> Box<dyn TaskDependency>;
}
assert_obj_safe!(Buildable);

impl Buildable for Box<dyn Buildable + '_> {
    fn get_build_dependencies(&self) -> Box<dyn TaskDependency> {
        self.as_ref().get_build_dependencies()
    }
}

impl<T: TaskDependency + Clone + Send + Sync + Debug + 'static> Buildable for T {
    fn get_build_dependencies(&self) -> Box<dyn TaskDependency> {
        let cloned = self.clone();
        Box::new(cloned)
    }
}
impl Buildable for DefaultTask {
    fn get_build_dependencies(&self) -> Box<dyn TaskDependency> {
        Box::new(self.task_id().clone())
    }
}

impl<B: Buildable> Buildable for Vec<B> {
    fn get_build_dependencies(&self) -> Box<dyn TaskDependency> {
        let mut set = TaskDependenciesSet::default();
        for buildable in self {
            set.add(buildable);
        }
        Box::new(set)
    }
}

/// Allows for adding "built by" info to non buildable objects
#[derive(Debug)]
pub struct BuiltBy<B: Buildable, T: Debug + Send + Sync> {
    built_by: B,
    value: T,
}

impl<B: Buildable, T: Debug + Send + Sync> DerefMut for BuiltBy<B, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<B: Buildable, T: Debug + Send + Sync> Deref for BuiltBy<B, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<B: Buildable, T: Debug + Send + Sync> AsMut<T> for BuiltBy<B, T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<B: Buildable, T: Debug + Send + Sync> AsRef<T> for BuiltBy<B, T> {
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<B: Buildable, T: Debug + Send + Sync> Buildable for BuiltBy<B, T> {
    fn get_build_dependencies(&self) -> Box<dyn TaskDependency> {
        self.built_by.get_build_dependencies()
    }
}

impl<B: Buildable, T: Debug + Send + Sync> BuiltBy<B, T> {
    /// Create a new buildable object
    pub fn new(built_by: B, value: T) -> Self {
        Self { built_by, value }
    }

    /// Makes this into the inner value
    pub fn into_inner(self) -> T {
        self.value
    }
}



/// The tasks that are required to be built by this project to make this object. If this is a task,
/// the task is also included.
pub trait TaskDependency {
    /// Gets the dependencies required to build this task
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError>;
}

assert_obj_safe!(TaskDependency);

impl TaskDependency for TaskId {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        println!("Attempting to get dependencies for {} in {}", self, project);
        let info = project
            .task_container
            .configure_task(self.clone(), project)?;
        println!("got info: {:#?}", info);
        let mut output: HashSet<_> = info.ordering.into_iter().map(|i| i.buildable).collect();
        output.insert(self.clone());
        Ok(output)
    }
}

impl TaskDependency for &str {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        let task_id: TaskId = project.find_task_id(self)?;
        task_id.get_dependencies(project)
    }
}

impl TaskDependency for String {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        self.as_str().get_dependencies(project)
    }
}

/// A set of task dependencies
#[derive(Default)]
pub struct TaskDependenciesSet(Vec<Box<dyn TaskDependency>>);

impl TaskDependenciesSet {
    pub fn add<T: Buildable>(&mut self, task: &T) {
        self.0.push(task.get_build_dependencies());
    }
}

impl TaskDependency for TaskDependenciesSet {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        let mut output = HashSet::new();
        for dep in &self.0 {
            output.extend(dep.get_dependencies(project)?);
        }
        Ok(output)
    }
}
