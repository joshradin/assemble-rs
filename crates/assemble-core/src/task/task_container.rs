use std::marker::PhantomData;
use crate::__export::TaskId;
use crate::{BuildResult, Executable, Project, Task};
use crate::project::ProjectResult;
use crate::task::ExecutableTask;
use crate::task::lazy_task::TaskProvider;

#[derive(Debug, Clone)]
pub struct TaskContainer;

impl TaskContainer {

    pub fn configure_task(&mut self, id: TaskId, project: &Project) -> ProjectResult<Box<dyn ExecutableTask>> { todo!() }
    pub fn register_task<T : Task + Send>(&mut self, id: TaskId) -> ProjectResult<TaskProvider<T>> { todo!() }
    pub fn get_tasks(&self) -> impl IntoIterator<Item=&TaskId> { vec![]}
}

