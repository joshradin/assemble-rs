use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use once_cell::sync::OnceCell;
use crate::__export::TaskId;
use crate::{BuildResult, Executable, Project, Task};
use crate::project::buildable::{Buildable, BuiltByContainer, IntoBuildable};
use crate::project::{ProjectError, ProjectResult, SharedProject};
use crate::task::{BuildableTask, ExecutableTask, ResolveExecutable, ResolveInnerTask, ResolveTask, TaskHandleFactory};
use crate::task::any_task::AnyTaskHandle;
use crate::task::lazy_task::TaskHandle;

#[derive(Debug)]
pub struct TaskContainer {
    shared: OnceCell<SharedProject>,
    handle_factory: OnceCell<TaskHandleFactory>,
    mapping: HashMap<TaskId, AnyTaskHandle>
}

impl TaskContainer {

    /// Creates a new task container. Tasks can not be registered until a project has been shared with
    /// the task container.
    pub fn new() -> Self {
        Self { shared: OnceCell::new(), handle_factory: OnceCell::new(), mapping: HashMap::new() }
    }

    /// Initialize the task factory
    pub fn init(&mut self, project: &SharedProject) {
        self.shared.set(project.clone()).expect("shared already set");
        self.handle_factory.set(
            TaskHandleFactory::new(project.clone())
        ).expect("factory already set");
    }

    pub fn register_task<T : Task + Send>(&mut self, id: TaskId) -> ProjectResult<TaskHandle<T>> { todo!() }

    pub fn get_task(&self, id: &TaskId) -> ProjectResult<AnyTaskHandle> {
        self.mapping
            .get(id)
            .ok_or(ProjectError::IdentifierMissing(id.clone()))
            .map(AnyTaskHandle::clone)
    }

    pub fn get_tasks(&self) -> impl IntoIterator<Item=&TaskId> { vec![]}

}

