//! The default tasks included in assemble

use crate::project::error::ProjectResult;
use crate::task::task_io::TaskIO;
use crate::task::up_to_date::UpToDate;
use crate::{BuildResult, Executable, Project, Task};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

mod help;
mod tasks_report;
mod wrapper;

use crate::task::create_task::CreateTask;
use crate::task::initialize_task::InitializeTask;
pub use help::Help;
pub use tasks_report::TaskReport;
pub use wrapper::WrapperTask;

/// A task that has no actions by default.
#[derive(Debug, Default)]
pub struct Empty;

impl UpToDate for Empty {}

impl InitializeTask for Empty {}

impl TaskIO for Empty {}

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

impl<T: Debug> Basic<T> {
    pub fn map(&self) -> &HashMap<String, T> {
        &self.map
    }

    pub fn map_mut(&mut self) -> &mut HashMap<String, T> {
        &mut self.map
    }
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

impl<T: Debug> TaskIO for Basic<T> {}

impl<T: Debug> Task for Basic<T> {
    fn task_action(_task: &mut Executable<Self>, _project: &Project) -> BuildResult {
        Ok(())
    }
}
