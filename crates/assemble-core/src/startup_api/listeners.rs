//! Provides listeners

use crate::startup_api::invocation::Assemble;
use crate::task::{ExecutableTask, TaskOutcome};
use crate::Executable;
use std::cell::Cell;
use std::fmt::Debug;

use crate::prelude::*;

/// A listener than can be added to a type.
pub trait Listener {
    type Listened;

    /// Add a listener to freight
    fn add_listener(self, freight: &mut Self::Listened);
}

/// A listener that listens for task execution
pub trait TaskExecutionListener : Debug {
    /// Listens for tasks to finish executing
    fn after_execute(&self, task: &dyn ExecutableTask, outcome: TaskOutcome) -> ProjectResult;
    /// Listens for tasks that are about to start executing
    fn before_execute(&self, task: &dyn ExecutableTask) -> ProjectResult;
}

impl<T: TaskExecutionListener + 'static> Listener for T {
    type Listened = Assemble;

    fn add_listener(self, freight: &mut Assemble) {
        freight.add_task_execution_listener(self)
    }
}

