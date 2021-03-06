use crate::exception::{BuildException, BuildResult};
use crate::project::{Project, ProjectError, ProjectResult};
use crate::task::task_container::TaskContainer;
use crate::utilities::AsAny;
use petgraph::data::Create;
use std::any::{type_name, Any};
use std::cell::{Ref, RefMut};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::sync::{Arc, RwLockWriteGuard};

mod executable;
pub use executable::Executable;
pub mod flags;

use crate::identifier::{ProjectId, TaskId};
use crate::private::Sealed;
use crate::project::buildable::{Buildable, BuiltByContainer, IntoBuildable};
use crate::properties::AnyProp;
use crate::work_queue::{WorkToken, WorkTokenBuilder};

pub mod task_container;
pub mod task_executor;
mod task_ordering;
pub use task_ordering::*;

mod lazy_task;
pub use lazy_task::*;

mod any_task;
use crate::task::flags::{OptionDeclaration, OptionDeclarations, OptionsDecoder};
use crate::task::up_to_date::UpToDate;
pub use any_task::AnyTaskHandle;

pub mod previous_work;
pub mod up_to_date;

pub trait TaskAction<T: Task>: Send {
    fn execute(&self, task: &mut Executable<T>, project: &Project) -> Result<(), BuildException>;
}

assert_obj_safe!(TaskAction<crate::defaults::tasks::Empty>);

impl<F, T> TaskAction<T> for F
where
    F: Fn(&mut Executable<T>, &Project) -> BuildResult,
    F: Send,
    T: Task,
{
    fn execute(&self, task: &mut Executable<T>, project: &Project) -> Result<(), BuildException> {
        (self)(task, project)
    }
}

pub struct Action<T: Task> {
    func: Box<dyn Fn(&mut Executable<T>, &Project) -> BuildResult + Send>,
}

impl<T: Task> Debug for Action<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Action<{}>", type_name::<T>())
    }
}

impl<T: Task> TaskAction<T> for Action<T> {
    fn execute(&self, task: &mut Executable<T>, project: &Project) -> Result<(), BuildException> {
        (self.func)(task, project)
    }
}

impl<T: Task> Action<T> {
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&mut Executable<T>, &Project) -> BuildResult + 'static,
        F: Send,
    {
        Self {
            func: Box::new(func),
        }
    }
}

/// Create tasks using a project.
pub trait CreateTask: Sized {
    /// Creates a new task. The using_id is the id of the task that's being created.
    fn new(using_id: &TaskId, project: &Project) -> ProjectResult<Self>;

    /// The default description for a Task
    fn description() -> String {
        String::new()
    }

    /// Gets an optional flags for this task.
    ///
    /// By defaults return `None`
    fn options_declarations() -> Option<OptionDeclarations> {
        None
    }

    /// Try to get values from a decoder.
    ///
    /// By default does not do anything.
    fn try_set_from_decoder(&mut self, decoder: &OptionsDecoder) -> ProjectResult<()> {
        Ok(())
    }
}

impl<T: Default> CreateTask for T {
    fn new(_: &TaskId, _: &Project) -> ProjectResult<Self> {
        Ok(T::default())
    }
}

pub trait InitializeTask<T: Task = Self> {
    /// Initialize tasks
    fn initialize(_task: &mut Executable<T>, _project: &Project) -> ProjectResult {
        Ok(())
    }
}

pub trait Task: UpToDate + InitializeTask + CreateTask + Sized + Debug {
    /// Check whether this task did work.
    ///
    /// By default, this is always true.
    fn did_work(&self) -> bool {
        true
    }

    /// The action that the task performs
    fn task_action(_task: &mut Executable<Self>, _project: &Project) -> BuildResult;
}

pub trait HasTaskId {
    fn task_id(&self) -> &TaskId;
}

pub trait BuildableTask: HasTaskId {
    fn built_by(&self) -> BuiltByContainer {
        let mut output = BuiltByContainer::new();
        for task_ordering in self.ordering() {
            match task_ordering.ordering_kind() {
                TaskOrderingKind::DependsOn => {
                    output.add(task_ordering.buildable().clone());
                }
                _ => {}
            }
        }
        output
    }

    fn ordering(&self) -> Vec<TaskOrdering>;
}

pub trait ExecutableTask: HasTaskId + Send {
    fn options_declarations(&self) -> Option<OptionDeclarations>;
    fn try_set_from_decoder(&mut self, decoder: &OptionsDecoder) -> ProjectResult<()>;

    fn execute(&mut self, project: &Project) -> BuildResult;

    fn did_work(&self) -> bool;
    fn up_to_date(&self) -> bool;

    fn group(&self) -> String;

    fn description(&self) -> String;
}

assert_obj_safe!(ExecutableTask);

pub trait FullTask: BuildableTask + ExecutableTask {}

impl Debug for Box<dyn FullTask> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Task {}", self.task_id())
    }
}

impl Display for Box<dyn FullTask> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Task {}", self.task_id())
    }
}

impl<F: BuildableTask + ExecutableTask> FullTask for F {}

assert_obj_safe!(FullTask);
