//! Define the buildable trait
//!
//! Buildable traits allows for easier constructing of project structures.
//!
//! # Buildable Types
//! - `String`, `&str`, or [`TaskId`](crate::__export::TaskId)
//! - Any type that implements [`TaskDependency`](TaskDependency)
//! - Any type that implements [`Buildable`](Buildable)
//! - [`FileCollection`](crate::file_collection::FileCollection)

use std::borrow::Borrow;
use crate::identifier::{Id, TaskId};
use crate::project::ProjectError;
use crate::{DefaultTask, Executable, project::Project, Task};
use itertools::Itertools;
use std::collections::HashSet;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use crate::task::Property;

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

impl Buildable for Arc<dyn Buildable + '_> {
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

impl<B: Buildable> Buildable for Vec<B> {
    fn get_build_dependencies(&self) -> Box<dyn TaskDependency> {
        let mut set = BuiltByHandler::default();
        for buildable in self {
            set.add(buildable);
        }
        Box::new(set)
    }
}

/// Allows for adding "built by" info to non buildable objects
#[derive(Debug)]
pub struct BuiltBy<T: Property + Debug> {
    built_by: Arc<dyn Buildable>,
    value: T,
}

impl<T: Property + Debug> Clone for BuiltBy<T> {
    fn clone(&self) -> Self {
        Self {
            built_by: self.built_by.clone(),
            value: self.value.clone()
        }
    }
}


impl<T: Property + Debug> DerefMut for BuiltBy<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T: Property + Debug> Deref for BuiltBy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        & self.value
    }
}


impl<T: Property + Debug> Buildable for BuiltBy<T> {
    fn get_build_dependencies(&self) -> Box<dyn TaskDependency> {
        self.built_by.get_build_dependencies()
    }
}

impl<T: Property + Debug> BuiltBy<T> {
    /// Create a new buildable object
    pub fn new<B: Buildable + 'static>(built_by: B, value: T) -> Self {
        Self { built_by: Arc::new(built_by), value }
    }

    /// Makes this into the inner value
    pub fn into_inner(self) -> T {
        self.value
    }

    /// Turns this built by into a built of a reference
    pub fn as_ref(&self) -> BuiltBy<&T> {
        BuiltBy {
            built_by: self.built_by.clone(),
            value: &self.value
        }
    }

    /// Gets a copy of the buildable
    pub fn built_by(&self) -> Arc<dyn Buildable> {
        self.built_by.clone()
    }

}



/// The tasks that are required to be built by this project to make this object. If this is a task,
/// the task is also included.
pub trait TaskDependency : Send + Sync {
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

impl TaskDependency for () {
    /// Will always return an empty set
    fn get_dependencies(&self, _project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        Ok(HashSet::new())
    }
}

impl TaskDependency for Box<dyn TaskDependency> {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        self.as_ref().get_dependencies(project)
    }
}

impl TaskDependency for Arc<dyn TaskDependency> {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        self.as_ref().get_dependencies(project)
    }
}

/// A set of task dependencies
#[derive(Default, Clone)]
pub struct BuiltByHandler(Vec<Arc<dyn TaskDependency>>);

impl BuiltByHandler {
    pub fn add<T: Buildable>(&mut self, task: &T) {
        self.0.push(Arc::new(task.get_build_dependencies()));
    }
    pub fn push<T: TaskDependency + 'static>(&mut self, deps: T) {
        self.0.push(Arc::new(deps));
    }
}

impl TaskDependency for BuiltByHandler {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        let mut output = HashSet::new();
        for dep in &self.0 {
            output.extend(dep.get_dependencies(project)?);
        }
        Ok(output)
    }
}
