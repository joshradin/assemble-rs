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
use crate::{Executable, Project, Task};
use crate::task::TaskId;

/// Represents something can be _built_ by the assemble project.
pub trait Buildable<E : Executable> : Send + Sync {
    /// Returns a dependency which contains the tasks which build this object.
    fn get_build_dependencies(&self) -> Box<dyn TaskDependency<E>>;
}


impl<E : Executable> Buildable<E> for Box<dyn Buildable<E> + '_> {
    fn get_build_dependencies(&self) -> Box<dyn TaskDependency<E>> {
        self.as_ref().get_build_dependencies()
    }
}

impl<E : Executable, T : TaskDependency<E> + Clone + Send + Sync + 'static> Buildable<E> for T {
    fn get_build_dependencies(&self) -> Box<dyn TaskDependency<E>> {
        let cloned = self.clone();
        Box::new(cloned)
    }
}

/// The tasks that are required to be built by this project to make this object. If this is a task,
/// the task is also included.
pub trait TaskDependency<E : Executable> {

    /// Gets the dependencies required to build this task
    fn get_dependencies(&self, project: &Project<E>) -> HashSet<TaskId>;
}


impl<E : Executable> TaskDependency<E> for E {
    fn get_dependencies(&self, project: &Project<E>) -> HashSet<TaskId> {
        self.task_dependencies()
            .into_iter()
            .map(|task| &task.buildable)
            .flat_map(|buildable|
                buildable.get_build_dependencies()
                    .get_dependencies(project)
            )
            .collect()
    }
}

impl<E : Executable> TaskDependency<E> for TaskId {
    fn get_dependencies(&self, project: &Project<E>) -> HashSet<TaskId> {
        let info = project.task_container.configure_task(self.clone(), project).expect("couldn't configure task");
        let mut output: HashSet<_> = info.ordering
            .into_iter()
            .map(|i| i.buildable)
            .collect();
        output.insert(self.clone());
        output
    }
}

impl<E : Executable> TaskDependency<E> for &str {
    fn get_dependencies(&self, project: &Project<E>) -> HashSet<TaskId> {
        let task_id = TaskId::new(*self);
        task_id.get_dependencies(project)
    }
}