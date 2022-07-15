use crate::exception::{BuildException, BuildResult};
use crate::project::{Project, ProjectError, ProjectResult};
use crate::task::task_container::TaskContainer;
use crate::utilities::AsAny;
use petgraph::data::Create;
use std::any::{Any, type_name};
use std::cell::{Ref, RefMut};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::sync::{Arc, RwLockWriteGuard};

pub mod task_container;
pub mod task_executor;

use crate::identifier::{ProjectId, TaskId};
use crate::private::Sealed;
use crate::project::buildable::{Buildable, BuiltByContainer, IntoBuildable};
use crate::properties::task_properties::TaskProperties;
use crate::properties::{AnyProp, FromProperties};
use crate::work_queue::{WorkToken, WorkTokenBuilder};

mod task_ordering;
pub use task_ordering::*;
mod executable;
pub use executable::Executable;

mod lazy_task;
pub use lazy_task::*;

pub trait TaskAction<T: Task> : Send {
    fn execute(&self, task: &mut Executable<T>, project: &Project) -> Result<(), BuildException>;
}

assert_obj_safe!(TaskAction<crate::task::Empty>);

impl <F, T> TaskAction<T> for F
    where F : Fn(&mut Executable<T>, &Project) -> BuildResult,
        F : Send,
        T : Task,
{
    fn execute(&self, task: &mut Executable<T>, project: &Project) -> Result<(), BuildException> {
        (self)(task, project)
    }
}

pub struct Action<T : Task> {
    func: Box<dyn Fn( &mut Executable<T>, &Project) -> BuildResult + Send>
}


impl<T: Task> Debug for Action<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Action<{}>", type_name::<T>())
    }
}

impl<T : Task> TaskAction<T> for Action<T> {
    fn execute(&self, task: &mut Executable<T>, project: &Project) -> Result<(), BuildException> {
        (self.func)(task, project)
    }
}

impl<T: Task> Action<T> {
    pub fn new<F>(func: F) -> Self
        where F : Fn( &mut Executable<T>, &Project) -> BuildResult + 'static,
            F : Send
    {
        Self { func: Box::new(func)}
    }
}

/// Create tasks using a project.
pub trait CreateTask {
    fn new(project: &Project) -> Self;
}

impl<T: Default> CreateTask for T {
    fn new(_: &Project) -> Self {
        T::default()
    }
}

pub trait InitializeTask: Task {
    /// Initialize tasks
    fn initialize(task: &mut Executable<Self>, project: &Project) -> ProjectResult;
}

impl<T: Default + Task> InitializeTask for T {
    fn initialize(_task: &mut Executable<Self>, _project: &Project) -> ProjectResult {
        Ok(())
    }
}

pub trait Task: CreateTask + Sized + Debug {
    /// The action that the task performs
    fn task_action(task: &mut Executable<Self>, project: &Project) -> BuildResult {
        Ok(())
    }
}

pub trait ExecutableTask: Send {
    fn task_id(&self) -> &TaskId;

    fn execute(&mut self, project: &Project) -> BuildResult;

    fn ordering(&self) -> Vec<TaskOrdering> {
        vec![]
    }
}

assert_obj_safe!(ExecutableTask);

/// A task that has no actions by default. This is the only task implemented in [assemble-core](crate)
#[derive(Debug, Default, Clone)]
pub struct Empty;

impl Task for Empty {}
