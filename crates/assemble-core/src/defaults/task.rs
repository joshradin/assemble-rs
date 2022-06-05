use crate::exception::{BuildException, BuildResult};
use crate::project::Project;
use crate::task::property::TaskProperties;
use crate::task::{
    Action, GetTaskAction, IntoTask, Task, TaskAction, TaskIdentifier, TaskMut, TaskOrdering,
};
use crate::utilities::AsAny;
use std::any::Any;
use std::cell::{RefCell, RefMut};
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::path::PathBuf;

#[derive(Default)]
pub struct DefaultTask {
    identifier: TaskIdentifier,
    actions: VecDeque<Box<dyn TaskAction>>,
    properties: RefCell<TaskProperties>,
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

impl Task for DefaultTask {
    fn task_id(&self) -> &TaskIdentifier {
        &self.identifier
    }

    fn actions(&self) -> Vec<&dyn TaskAction> {
        self.actions.iter().map(|action| action.as_ref()).collect()
    }

    fn properties(&self) -> RefMut<TaskProperties> {
        self.properties.borrow_mut()
    }

    fn task_dependencies(&self) -> Vec<&TaskOrdering> {
        self.task_dependencies.iter().collect()
    }
}

impl TaskMut for DefaultTask {
    fn set_task_id(&mut self, id: TaskIdentifier) {
        self.identifier = id;
    }

    fn first<A: TaskAction + 'static>(&mut self, action: A) {
        self.actions.push_front(Box::new(action))
    }

    fn last<A: TaskAction + 'static>(&mut self, action: A) {
        self.actions.push_back(Box::new(action))
    }

    fn depends_on<I: Into<TaskIdentifier>>(&mut self, identifier: I) {
        self.task_dependencies
            .push(TaskOrdering::DependsOn(identifier.into()))
    }
}

pub struct Echo {
    pub string: String,
}

impl GetTaskAction for Echo {
    fn task_action(task: &dyn Task, project: &Project) -> BuildResult {
        let string = task
            .properties()
            .get::<String, _>("string")
            .map(|p| p.to_string())
            .unwrap();

        println!("{}", string);
        Ok(())
    }
}

impl IntoTask for Echo {
    type Task = DefaultTask;
    type Error = ();

    fn create() -> Self {
        Self {
            string: String::new(),
        }
    }

    fn default_task() -> Self::Task {
        DefaultTask::default()
    }

    fn inputs(&self) -> Vec<&str> {
        vec![]
    }

    fn outputs(&self) -> Vec<&str> {
        vec![]
    }

    fn set_properties(&self, properties: &mut TaskProperties) {
        properties.set("string", self.string.to_owned());
    }
}

pub struct Exec {
    id: TaskIdentifier,
}
