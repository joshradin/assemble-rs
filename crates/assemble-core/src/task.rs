use crate::exception::{BuildException, BuildResult};
use crate::project::{Project, ProjectError};
use crate::task::task_container::TaskContainer;
use crate::utilities::AsAny;
use std::any::Any;
use std::cell::{Ref, RefMut};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::sync::RwLockWriteGuard;

pub mod property;
pub mod task_container;
pub mod task_executor;

use crate::internal::macro_helpers::WriteIntoProperties;

use crate::identifier::TaskId;
use crate::private::Sealed;
use crate::project::buildable::{Buildable, TaskDependency};
use crate::work_queue::{WorkToken, WorkTokenBuilder};
use crate::DefaultTask;
use property::FromProperties;
pub use property::*;
use crate::properties::Prop;

pub trait TaskAction<T: Executable = DefaultTask> {
    fn execute(&self, task: &T, project: &Project) -> Result<(), BuildException>;
}

assert_obj_safe!(TaskAction);

pub struct Action<F, T: Executable> {
    func: F,
    _task: PhantomData<T>,
}

impl<F, T> TaskAction<T> for Action<F, T>
where
    T: Executable,
    F: Fn(&T, &Project) -> Result<(), BuildException>,
{
    fn execute(&self, task: &T, project: &Project) -> Result<(), BuildException> {
        (&self.func)(task, project)
    }
}

impl<F, T> Action<F, T>
where
    T: Executable,
    F: Fn(&T, &Project) -> Result<(), BuildException>,
{
    pub fn new(func: F) -> Self {
        Self {
            func,
            _task: PhantomData,
        }
    }
}

/// An executable task are what Projects actually run. This trait can not be implemented outside of this crate.
pub trait Executable: Sealed + Sized + Send + Sync + Debug {
    fn task_id(&self) -> &TaskId;

    fn actions(&self) -> Vec<&dyn TaskAction<Self>>;

    fn properties(&self) -> RwLockWriteGuard<TaskProperties>;

    fn task_dependencies(&self) -> Vec<&GenericTaskOrdering>;

    fn execute(&mut self, project: &Project) -> BuildResult;
}

/// Provides mutable access to an ExecutableTask.
pub trait ExecutableTaskMut: Executable {
    fn set_task_id(&mut self, id: TaskId);

    fn first<A: TaskAction<Self> + Send + Sync + 'static>(&mut self, action: A);
    fn last<A: TaskAction<Self> + Send + Sync + 'static>(&mut self, action: A);

    fn depends_on<B: Buildable + 'static>(&mut self, buildable: B);
    fn connect_to<B: Buildable + 'static>(&mut self, ordering: TaskOrdering<B>);
}

pub trait GetTaskAction<T: Executable + Send> {
    fn task_action(task: &T, project: &Project) -> BuildResult;
    fn get_task_action(&self) -> fn(&T, &Project) -> BuildResult {
        Self::task_action
    }

    fn as_action(&self) -> Action<fn(&T, &Project) -> BuildResult, T> {
        Action::new(self.get_task_action())
    }
}

pub trait DynamicTaskAction: Task {
    fn exec(&mut self, project: &Project) -> BuildResult;
}

impl<T: DynamicTaskAction + WriteIntoProperties + FromProperties> GetTaskAction<T::ExecutableTask>
    for T
{
    fn task_action(task: &T::ExecutableTask, project: &Project) -> BuildResult {
        let properties = &mut *task.properties();
        let mut my_task = T::from_properties(properties);
        let result = T::exec(&mut my_task, project);
        my_task.set_properties(properties);
        result
    }
}

pub trait Task: GetTaskAction<Self::ExecutableTask> + Send + Sync {
    type ExecutableTask: ExecutableTaskMut + 'static + Send + Sync;

    /// Create a new task with this name
    fn create() -> Self;

    /// Get a copy of the default tasks
    fn default_task() -> Self::ExecutableTask;

    fn inputs(&self) -> Vec<&str>;
    fn outputs(&self) -> Vec<&str>;

    fn set_properties(&self, properties: &mut TaskProperties);

    fn into_task(self) -> Result<Self::ExecutableTask, ProjectError>
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

/// Represents some sort of order between a task and something that can be buiklt
#[derive(Eq, PartialEq)]
pub struct TaskOrdering<B>
where
    B: TaskDependency,
{
    pub buildable: B,
    pub ordering_type: TaskOrderingKind,
    _data: PhantomData<()>,
}

impl<B> Debug for TaskOrdering<B>
where
    B: Buildable,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {:?}", self.ordering_type, self.buildable)
    }
}

impl<B> Display for TaskOrdering<B>
where
    B: Buildable + Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.ordering_type, self.buildable)
    }
}

impl<B> TaskOrdering<B>
where
    B: Buildable,
{
    pub fn new(buildable: B, ordering_type: TaskOrderingKind) -> Self {
        Self {
            buildable,
            ordering_type,
            _data: PhantomData,
        }
    }

    pub fn as_task_ids(
        &self,
        project: &Project,
    ) -> Result<Vec<TaskOrdering<TaskId>>, ProjectError> {
        let task_deps = self.buildable.get_build_dependencies();
        let set = match task_deps.get_dependencies(project) {
            Ok(set) => {
                set
            }
            Err(e) => {
                eprintln!("got error {} while trying to find dependencies of {:?}", e, self.buildable);
                return Err(e);
            }
        };
        Ok(set
            .into_iter()
            .map(|id| TaskOrdering::new(id, self.ordering_type))
            .collect())
    }

    pub fn depends_on(buildable: B) -> Self {
        Self::new(buildable, TaskOrderingKind::DependsOn)
    }

    pub fn map<F, B2>(self, transform: F) -> TaskOrdering<B2>
    where
        B2: Buildable,
        F: Fn(B) -> B2,
    {
        let TaskOrdering {
            buildable,
            ordering_type,
            _data,
        } = self;
        let transformed = (transform)(buildable);
        TaskOrdering::new(transformed, ordering_type)
    }
}

impl<B> Clone for TaskOrdering<B>
where
    B: Buildable + Clone,
{
    fn clone(&self) -> Self {
        Self {
            buildable: self.buildable.clone(),
            ordering_type: self.ordering_type,
            _data: Default::default(),
        }
    }
}

pub type GenericTaskOrdering<'p> = TaskOrdering<Box<dyn Buildable + 'p>>;

/// How the tasks should be ordered.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TaskOrderingKind {
    DependsOn,
    FinalizedBy,
    RunsAfter,
    RunsBefore,
}

impl Display for TaskOrderingKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub trait ResolveTaskIdentifier {
    fn resolve_task(&self, project: &Project) -> TaskId {
        self.try_resolve_task(project).unwrap()
    }
    fn try_resolve_task(&self, project: &Project) -> Result<TaskId, ProjectError>;
}

assert_obj_safe!(ResolveTaskIdentifier);

pub struct TaskOptions<'project, T: Executable> {
    task_ordering: Vec<GenericTaskOrdering<'project>>,
    do_first: Vec<Box<dyn TaskAction<T>>>,
    do_last: Vec<Box<dyn TaskAction<T>>>,
    extra_properties: TaskProperties,
}

impl<E: Executable> Default for TaskOptions<'_, E> {
    fn default() -> Self {
        Self {
            task_ordering: vec![],
            do_first: vec![],
            do_last: vec![],
            extra_properties: TaskProperties::default()
        }
    }
}

impl<'p, T: Executable> TaskOptions<'p, T> {
    pub fn depend_on<R: 'p + Buildable>(&mut self, object: R) {
        self.task_ordering
            .push(TaskOrdering::depends_on(Box::new(object)))
    }

    pub fn first<A: TaskAction<T> + 'static>(&mut self, action: A) {
        self.do_first.push(Box::new(action));
    }

    pub fn do_first<F>(&mut self, func: F)
    where
        F: 'static + Fn(&T, &Project) -> BuildResult,
        T: 'static,
    {
        self.first(Action::new(func))
    }

    pub fn last<A: TaskAction<T> + 'static>(&mut self, action: A) {
        self.do_last.push(Box::new(action));
    }

    /// Allows for adding extra properties to tasks.
    pub fn properties(&mut self) -> &mut TaskProperties {
        &mut self.extra_properties
    }
}

impl<T: ExecutableTaskMut + 'static> TaskOptions<'static, T> {
    pub fn apply_to(self, project: &Project, task: &mut T) -> Result<(), ProjectError> {
        for ordering in self.task_ordering {
            task.connect_to(ordering)
        }

        Ok(())
    }
}

pub struct Configure<'a, T: Task> {
    delegate: &'a mut T,
    options: &'a mut TaskOptions<'a, T::ExecutableTask>,
}

impl<'a, T: Task> Configure<'a, T> {
    pub fn new(delegate: &'a mut T, options: &'a mut TaskOptions<'a, T::ExecutableTask>) -> Self {
        Self { delegate, options }
    }
}

impl<'a, T: Task> DerefMut for Configure<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.delegate
    }
}

impl<'a, T: Task> Deref for Configure<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.delegate
    }
}

impl ResolveTaskIdentifier for &str {
    fn try_resolve_task(&self, project: &Project) -> Result<TaskId, ProjectError> {
        project.resolve_task_id(self)
    }
}

/// A task that has no actions by default. This is the only task implemented in [assemble-core](crate)
#[derive(Debug, Default)]
pub struct Empty;

impl GetTaskAction<DefaultTask> for Empty {
    fn task_action(task: &DefaultTask, project: &Project) -> BuildResult {
        no_action(task, project)
    }
}

impl Task for Empty {
    type ExecutableTask = DefaultTask;

    fn create() -> Self {
        Self
    }

    fn default_task() -> Self::ExecutableTask {
        DefaultTask::default()
    }

    fn inputs(&self) -> Vec<&str> {
        vec![]
    }

    fn outputs(&self) -> Vec<&str> {
        vec![]
    }

    fn set_properties(&self, _properties: &mut TaskProperties) {}


}

/// A no-op task action
#[inline]
pub fn no_action<T: Executable>(_: &T, _: &Project) -> BuildResult {
    Ok(())
}
