//! The default tasks included in assemble

use crate::project::ProjectResult;
use crate::task::up_to_date::UpToDate;
use crate::task::{CreateTask, InitializeTask};
use crate::{BuildResult, Executable, Project, Task};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

mod tasks_report;
pub use tasks_report::TaskReport;

mod help;
pub use help::Help;

/// A task that has no actions by default.
#[derive(Debug, Default)]
pub struct Empty;

impl UpToDate for Empty {}

impl InitializeTask for Empty {}

impl Task for Empty {
    fn task_action(_task: &mut Executable<Self>, _project: &Project) -> BuildResult {
        Ok(())
    }
}

/// A basic task is a task that by default only contains a hashmap of data.
#[derive(Debug)]
pub struct Basic<T: Debug> {
    map: HashMap<String, T>,
}

impl<T: Debug> Default for Basic<T> {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

impl<T: Debug> UpToDate for Basic<T> {}

impl<T: Debug> InitializeTask for Basic<T> {}

impl<T: Debug> Task for Basic<T> {
    fn task_action(_task: &mut Executable<Self>, _project: &Project) -> BuildResult {
        Ok(())
    }
}
