use crate::exception::BuildResult;
use crate::project::Project;

use parking_lot::RwLock;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

use crate::identifier::TaskId;

use crate::project::buildable::BuiltByContainer;

pub mod action;
mod any_task;
pub mod create_task;
mod executable;
pub mod flags;
pub mod initialize_task;
mod lazy_task;
pub mod task_container;
pub mod task_executor;
pub mod task_io;
mod task_ordering;
pub mod up_to_date;
pub mod work_handler;

use crate::project::error::ProjectResult;
use crate::task::flags::{OptionDeclarations, OptionsDecoder};
use crate::task::up_to_date::UpToDate;
pub use any_task::AnyTaskHandle;
use create_task::CreateTask;
pub use executable::{force_rerun, Executable};
use initialize_task::InitializeTask;
pub use lazy_task::*;
use task_io::TaskIO;

pub use task_ordering::*;

/// The outcome of task.
#[derive(Debug, Clone)]
pub enum TaskOutcome {
    /// the task executed successfully
    Executed,
    /// The task was skipped
    Skipped,
    /// The task was up to date
    UpToDate,
    /// The task had no source
    NoSource,
    /// The task failed
    Failed,
}

pub trait Task: UpToDate + InitializeTask + CreateTask + TaskIO + Sized + Debug {
    /// Check whether this task did work.
    ///
    /// By default, this is always true.
    fn did_work(&self) -> bool {
        true
    }

    /// The action that the task performs
    fn task_action(_task: &mut Executable<Self>, _project: &Project) -> BuildResult;
}

/// Represents an object that has a task id
pub trait HasTaskId {
    /// Gets the task id
    fn task_id(&self) -> TaskId;
}

/// Tasks that are buildable are able to produce a [`BuiltByContainer`][0] that
/// describes what tasks are required in order to run the task.
///
/// [0]: BuiltByContainer
pub trait BuildableTask: HasTaskId {
    /// Gets the tasks that this task depends on
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

    /// Gets the total ordering associated with the task. This includes all types of ordering,
    /// including those that aren't strict dependencies.
    ///
    /// See [`TaskOrdering`](TaskOrdering) for more information
    fn ordering(&self) -> Vec<TaskOrdering>;
}

/// A object safe generic trait for executing tasks
pub trait ExecutableTask: HasTaskId + Send + Sync {
    /// Get the options declaration for this task
    fn options_declarations(&self) -> Option<OptionDeclarations>;

    /// Try to set values from a decoder
    fn try_set_from_decoder(&mut self, decoder: &OptionsDecoder) -> ProjectResult<()>;

    /// Executes the task, with a given project
    fn execute(&mut self, project: &Project) -> BuildResult;

    /// Checks if this task did work
    fn did_work(&self) -> bool;
    /// Check if this task marked itself as up to date
    fn task_up_to_date(&self) -> bool;

    /// Gets the group of the task
    fn group(&self) -> String;

    /// Gets the description of the task
    fn description(&self) -> String;
}

assert_obj_safe!(ExecutableTask);

/// A full task is buildable and executable.
pub trait FullTask: BuildableTask + ExecutableTask + Send + Sync {}

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

impl HasTaskId for Box<dyn FullTask> {
    fn task_id(&self) -> TaskId {
        (**self).task_id()
    }
}

impl ExecutableTask for Box<dyn FullTask> {
    fn options_declarations(&self) -> Option<OptionDeclarations> {
        (**self).options_declarations()
    }

    fn try_set_from_decoder(&mut self, decoder: &OptionsDecoder) -> ProjectResult<()> {
        (**self).try_set_from_decoder(decoder)
    }

    fn execute(&mut self, project: &Project) -> BuildResult {
        (**self).execute(project)
    }

    fn did_work(&self) -> bool {
        (**self).did_work()
    }

    fn task_up_to_date(&self) -> bool {
        (**self).task_up_to_date()
    }

    fn group(&self) -> String {
        (**self).group()
    }

    fn description(&self) -> String {
        (**self).description()
    }
}

impl<E: ExecutableTask> HasTaskId for Arc<RwLock<E>> {
    fn task_id(&self) -> TaskId {
        self.read().task_id()
    }
}

impl<E: ExecutableTask + Send + Sync> ExecutableTask for Arc<RwLock<E>> {
    fn options_declarations(&self) -> Option<OptionDeclarations> {
        self.read().options_declarations()
    }

    fn try_set_from_decoder(&mut self, decoder: &OptionsDecoder) -> ProjectResult<()> {
        self.write().try_set_from_decoder(decoder)
    }

    fn execute(&mut self, project: &Project) -> BuildResult {
        self.write().execute(project)
    }

    fn did_work(&self) -> bool {
        self.read().did_work()
    }

    fn task_up_to_date(&self) -> bool {
        self.read().task_up_to_date()
    }

    fn group(&self) -> String {
        self.read().group()
    }

    fn description(&self) -> String {
        self.read().description()
    }
}

impl Debug for Box<dyn FullTask + Send + Sync> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Task {}", self.task_id())
    }
}

impl Display for Box<dyn FullTask + Send + Sync> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Task {}", self.task_id())
    }
}

impl<F: BuildableTask + ExecutableTask> FullTask for F {}

assert_obj_safe!(FullTask);
