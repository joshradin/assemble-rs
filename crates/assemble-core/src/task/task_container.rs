use crate::__export::TaskId;
use crate::identifier::TaskIdFactory;

use crate::project::error::{ProjectError, ProjectResult};
use crate::project::shared::WeakSharedProject;
use crate::task::any_task::AnyTaskHandle;
use crate::task::lazy_task::TaskHandle;
use crate::task::TaskHandleFactory;
use crate::{Executable, Project, Task};
use once_cell::sync::OnceCell;

use crate::error::PayloadError;
use crate::project::finder::TaskPath;
use crate::project::shared::SharedProject;
use itertools::Itertools;
use std::collections::HashMap;
use std::fmt::Debug;

#[derive(Debug)]
pub struct TaskContainer {
    shared: OnceCell<WeakSharedProject>,
    task_id_factory: TaskIdFactory,
    handle_factory: OnceCell<TaskHandleFactory>,
    mapping: HashMap<TaskId, AnyTaskHandle>,
}

impl TaskContainer {
    /// Creates a new task container. Tasks can not be registered until a project has been shared with
    /// the task container.
    pub fn new(id_factory: TaskIdFactory) -> Self {
        Self {
            shared: OnceCell::new(),
            task_id_factory: id_factory,
            handle_factory: OnceCell::new(),
            mapping: HashMap::new(),
        }
    }

    /// Initialize the task factory
    pub(crate) fn init(&mut self, project: &WeakSharedProject) {
        self.shared
            .set(project.clone())
            .expect("shared already set");
        self.handle_factory
            .set(TaskHandleFactory::new(project.clone()))
            .expect("factory already set");
    }

    #[inline]
    fn handle_factory(&self) -> &TaskHandleFactory {
        self.handle_factory
            .get()
            .expect("task handle should be set")
    }

    #[inline]
    fn shared_project(&self) -> SharedProject {
        let weak = self.shared.get().unwrap();
        weak.clone().upgrade().expect("should be not weak")
    }

    pub fn register_task<T: Task + Send + Sync + Debug + 'static>(
        &mut self,
        id: &str,
    ) -> ProjectResult<TaskHandle<T>> {
        let id = self.task_id_factory.create(id).map_err(PayloadError::new)?;

        if self.mapping.contains_key(&id) {
            panic!("Task with id {} already registered", id);
        }

        let handle = self
            .handle_factory()
            .create_handle::<T>(id.clone())
            .map_err(PayloadError::new)?;
        let any_task_handle = AnyTaskHandle::new(handle.clone());
        self.mapping.insert(id, any_task_handle);
        Ok(handle)
    }
    pub fn register_task_with<
        T: Task + Send + Sync + Debug + 'static,
        F: 'static + Send + FnOnce(&mut Executable<T>, &Project) -> ProjectResult,
    >(
        &mut self,
        id: &str,
        config: F,
    ) -> ProjectResult<TaskHandle<T>> {
        let mut handle = self.register_task::<T>(id)?;
        handle.configure_with(config)?;
        Ok(handle)
    }

    /// Get all tasks registered to this task container
    pub fn get_tasks(&self) -> impl IntoIterator<Item = &TaskId> {
        self.mapping.keys()
    }

    /// Get a task
    pub fn get_task(&self, id: &TaskId) -> Option<&AnyTaskHandle> {
        self.mapping.get(id)
    }
}
