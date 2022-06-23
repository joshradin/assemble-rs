use crate::exception::{BuildException, BuildResult};
use crate::project::Project;
use crate::task::property::TaskProperties;
use crate::task::{
    Action, ExecutableTask, ExecutableTaskMut, GetTaskAction, Task, TaskAction, TaskId,
    TaskOrdering,
};
use crate::utilities::AsAny;
use std::any::Any;
use std::cell::{RefCell, RefMut};
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::fmt::{Debug, Display, Formatter};
use std::path::PathBuf;
use std::sync::{RwLock, RwLockWriteGuard};

#[derive(Default)]
pub struct DefaultTask {
    identifier: TaskId,
    actions: VecDeque<Box<dyn TaskAction<DefaultTask> + Send + Sync>>,
    properties: RwLock<TaskProperties>,
    task_dependencies: Vec<TaskOrdering>,
}

impl DefaultTask {
    pub fn new(identifier: TaskId) -> Self {
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
    fn task_id(&self) -> &TaskId {
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
            match action.execute(self, project) {
                Ok(_) => {}
                Err(exception) => {
                    match exception {
                        BuildException::StopAction => {}
                        BuildException::StopTask => {
                            return Ok(())
                        }
                        e => {
                            return Err(e);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

impl ExecutableTaskMut for DefaultTask {
    fn set_task_id(&mut self, id: TaskId) {
        self.identifier = id;
    }

    fn first<A: TaskAction + Send + Sync +'static>(&mut self, action: A) {
        self.actions.push_front(Box::new(action))
    }

    fn last<A: TaskAction + Send + Sync+ 'static>(&mut self, action: A) {
        self.actions.push_back(Box::new(action))
    }

    fn depends_on<I: Into<TaskId>>(&mut self, identifier: I) {
        self.task_dependencies
            .push(TaskOrdering::DependsOn(identifier.into()))
    }
}

impl Debug for DefaultTask {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Task '{}'", self.identifier)
    }
}

impl Display for DefaultTask {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
