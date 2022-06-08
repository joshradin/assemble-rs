use crate::exception::{BuildException, BuildResult};
use crate::project::Project;
use crate::task::task_container::TaskContainer;
use crate::utilities::AsAny;
use std::any::Any;
use std::cell::{Ref, RefMut};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::ops::{Index, IndexMut};

pub mod property;
pub mod task_container;

use crate::internal::macro_helpers::WriteIntoProperties;


use property::FromProperties;
pub use property::*;

pub trait TaskAction {
    fn execute(&self, task: &dyn Task, project: &Project) -> Result<(), BuildException>;
}

assert_obj_safe!(TaskAction);

pub struct Action<F> {
    func: F,
}

impl<F> TaskAction for Action<F>
where
    F: Fn(&dyn Task, &Project) -> Result<(), BuildException>,
{
    fn execute(&self, task: &dyn Task, project: &Project) -> Result<(), BuildException> {
        (&self.func)(task, project)
    }
}

impl<F> Action<F>
where
    F: Fn(&dyn Task, &Project) -> Result<(), BuildException>,
{
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

pub trait Task {
    fn task_id(&self) -> &TaskIdentifier;

    fn actions(&self) -> Vec<&dyn TaskAction>;

    fn properties(&self) -> RefMut<TaskProperties>;

    fn task_dependencies(&self) -> Vec<&TaskOrdering>;
}

pub trait TaskMut: Task {
    fn set_task_id(&mut self, id: TaskIdentifier);

    fn first<A: TaskAction + 'static>(&mut self, action: A);
    fn last<A: TaskAction + 'static>(&mut self, action: A);

    fn depends_on<I: Into<TaskIdentifier>>(&mut self, identifier: I);
}

pub trait GetTaskAction {
    fn task_action(task: &dyn Task, project: &Project) -> BuildResult;
    fn get_task_action(&self) -> fn(&dyn Task, &Project) -> BuildResult {
        Self::task_action
    }

    fn as_action(&self) -> Action<fn(&dyn Task, &Project) -> BuildResult> {
        Action::new(self.get_task_action())
    }
}

pub trait DynamicTaskAction {
    fn exec(&mut self, project: &Project) -> BuildResult;
}

impl<T: DynamicTaskAction + WriteIntoProperties + FromProperties> GetTaskAction for T {
    fn task_action(task: &dyn Task, project: &Project) -> BuildResult {
        let properties = &mut *task.properties();
        let mut my_task = T::from_properties(properties);
        let result = T::exec(&mut my_task, project);
        my_task.set_properties(properties);
        result
    }
}

pub trait IntoTask: GetTaskAction {
    type Task: TaskMut;
    type Error;

    /// Create a new task with this name
    fn create() -> Self;

    /// Get a copy of the default tasks
    fn default_task() -> Self::Task;

    fn inputs(&self) -> Vec<&str>;
    fn outputs(&self) -> Vec<&str>;

    fn set_properties(&self, properties: &mut TaskProperties);

    fn into_task(self) -> Result<Self::Task, Self::Error>
    where
        Self: Sized,
    {
        let mut output = Self::default_task();
        let mut properties = output.properties();
        self.set_properties(&mut *properties);
        drop(properties);
        output.first(self.as_action());

        Ok(output)
    }
}

#[derive(Default, Debug, Eq, PartialEq, Clone, Hash)]
pub struct TaskIdentifier(String);

impl TaskIdentifier {
    pub fn new<S: TryInto<TaskIdentifier, Error = InvalidTaskIdentifier>>(name: S) -> Self {
        name.try_into().unwrap()
    }
}

impl TryFrom<&str> for TaskIdentifier {
    type Error = InvalidTaskIdentifier;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(TaskIdentifier(value.to_string()))
    }
}

#[derive(Debug)]
pub struct InvalidTaskIdentifier(String);

impl Display for InvalidTaskIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid Task Identifier {:?}", self.0)
    }
}

impl Error for InvalidTaskIdentifier {}

#[derive(Debug)]
pub enum TaskOrdering {
    DependsOn(TaskIdentifier),
    FinalizedBy(TaskIdentifier),
    RunsAfter(TaskIdentifier),
    RunsBefore(TaskIdentifier),
}

pub trait ResolveTaskIdentifier<'p> {
    fn resolve_task(&self, project: &Project) -> TaskIdentifier;
}

assert_obj_safe!(ResolveTaskIdentifier<'static>);

#[derive(Default)]
pub struct TaskOptions<'project> {
    task_ordering: Vec<(
        TaskOrdering,
        Box<(dyn 'project + ResolveTaskIdentifier<'project>)>,
    )>,
    do_first: Vec<Box<dyn TaskAction>>,
    do_last: Vec<Box<dyn TaskAction>>,
}

impl<'p> TaskOptions<'p> {
    pub fn depend_on<R: 'p + ResolveTaskIdentifier<'p>>(&mut self, object: R) {
        self.task_ordering.push((
            TaskOrdering::DependsOn(TaskIdentifier::default()),
            Box::new(object),
        ))
    }

    pub fn first<A: TaskAction + 'static>(&mut self, action: A) {
        self.do_first.push(Box::new(action));
    }
    pub fn last<A: TaskAction + 'static>(&mut self, action: A) {
        self.do_last.push(Box::new(action));
    }
}

impl TaskOptions<'_> {
    pub fn apply_to<T: TaskMut>(self, project: &Project, task: &mut T) {
        for (ordering, resolver) in self.task_ordering {
            let task_id = resolver.resolve_task(project);
            match ordering {
                TaskOrdering::DependsOn(_) => {
                    task.depends_on(task_id);
                }
                TaskOrdering::FinalizedBy(_) => {}
                TaskOrdering::RunsAfter(_) => {}
                TaskOrdering::RunsBefore(_) => {}
            }
        }
    }
}

impl ResolveTaskIdentifier<'_> for &str {
    fn resolve_task(&self, project: &Project) -> TaskIdentifier {
        todo!()
    }
}
