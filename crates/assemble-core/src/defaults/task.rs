use crate::exception::{BuildException, BuildResult};
use crate::project::Project;
use crate::task::property::TaskProperties;
use crate::task::{
    Action, ExecutableTask, ExecutableTaskMut, GetTaskAction, Task, TaskAction, TaskIdentifier,
    TaskOrdering,
};
use crate::utilities::AsAny;
use std::any::Any;
use std::cell::{RefCell, RefMut};
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::{RwLock, RwLockWriteGuard};

#[derive(Default)]
pub struct DefaultTask {
    identifier: TaskIdentifier,
    actions: VecDeque<Box<dyn TaskAction<DefaultTask> + Send + Sync>>,
    properties: RwLock<TaskProperties>,
    task_dependencies: Vec<TaskOrdering>,
}

impl DefaultTask {
    pub fn new(identifier: TaskIdentifier) -> Self {
        Self {
            identifier,
            ..Default::default()
        }
    }
}

impl AsAny for DefaultTask {
    fn as_any(&self) -> &(dyn Any + '_) {
        self as &dyn Any
    }
}

impl ExecutableTask for DefaultTask {
    fn task_id(&self) -> &TaskIdentifier {
        &self.identifier
    }

    fn actions(&self) -> Vec<&dyn TaskAction<Self>> {
        self.actions.iter().map(|action| action.as_ref())
            .map(|sync| {
                sync as &dyn TaskAction<Self>
            }).collect()
    }

    fn properties(&self) -> RwLockWriteGuard<TaskProperties> {
        self.properties.write().unwrap()
    }

    fn task_dependencies(&self) -> Vec<&TaskOrdering> {
        self.task_dependencies.iter().collect()
    }

    fn execute(&mut self, project: &Project<Self>) -> BuildResult {
        let collected = self.actions.drain(..).collect::<Vec<_>>();
        for action in collected {
            action.execute(self, project)?
        }
        Ok(())
    }
}

impl ExecutableTaskMut for DefaultTask {
    fn set_task_id(&mut self, id: TaskIdentifier) {
        self.identifier = id;
    }

    fn first<A: TaskAction + Send + Sync +'static>(&mut self, action: A) {
        self.actions.push_front(Box::new(action))
    }

    fn last<A: TaskAction + Send + Sync+ 'static>(&mut self, action: A) {
        self.actions.push_back(Box::new(action))
    }

    fn depends_on<I: Into<TaskIdentifier>>(&mut self, identifier: I) {
        self.task_dependencies
            .push(TaskOrdering::DependsOn(identifier.into()))
    }
}
