//! Define the buildable trait
//!
//! Buildable traits allows for easier constructing of project structures.
//!
//! # Buildable Types
//! - `String`, `&str`, or [`TaskId`](crate::__export::TaskId)
//! - Any type that implements [`TaskDependency`](TaskDependency)
//! - Any type that implements [`Buildable`](Buildable)
//! - [`FileCollection`](crate::file_collection::FileCollection)

use std::collections::HashSet;
use itertools::Itertools;
use crate::{Executable, project::Project, Task};
use crate::identifier::TaskId;
use crate::project::ProjectError;

/// Represents something can be _built_ by the assemble project.
pub trait Buildable : Send + Sync {

    /// Returns a dependency which contains the tasks which build this object.
    fn get_build_dependencies(&self) -> Box<dyn TaskDependency>;
}


impl Buildable for Box<dyn Buildable + '_> {
    fn get_build_dependencies(&self) -> Box<dyn TaskDependency> {
        self.as_ref().get_build_dependencies()
    }
}

impl<T : TaskDependency + Clone + Send + Sync + 'static> Buildable for T {
    fn get_build_dependencies(&self) -> Box<dyn TaskDependency> {
        let cloned = self.clone();
        Box::new(cloned)
    }
}

assert_obj_safe!(Buildable);

/// The tasks that are required to be built by this project to make this object. If this is a task,
/// the task is also included.
pub trait TaskDependency {

    /// Gets the dependencies required to build this task
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError>;
}

assert_obj_safe!(TaskDependency);

impl TaskDependency for TaskId {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        let info = project.task_container.configure_task(self.clone(), project)?;
        let mut output: HashSet<_> = info.ordering
            .into_iter()
            .map(|i| i.buildable)
            .collect();
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