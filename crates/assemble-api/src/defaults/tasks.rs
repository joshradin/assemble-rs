use crate::task::{Task, TaskAction, TaskIdentifier, TaskMut, TaskOrdering, TaskProperties};
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

#[assemble_macros::task(action = exec_action)]
struct ExecTask {
    working_dir: PathBuf,
    executable: String,
    args: Vec<String>,
}
