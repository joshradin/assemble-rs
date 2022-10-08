//! Provides listeners for freight

use assemble_core::Executable;
use assemble_core::project::ProjectResult;
use assemble_core::task::{ExecutableTask, TaskOutcome};
use crate::FreightInner;

/// A listener than can be added to freight
pub trait Listener {

    /// Add a listener to freight
    fn add_listener(self, freight: &mut FreightInner);
}

/// A listener that listens for task execution
pub trait TaskExecutionListener {
    fn after_execute(&self, task: &dyn ExecutableTask, outcome: TaskOutcome) -> ProjectResult;
    fn before_execute(&self, task: &dyn ExecutableTask) -> ProjectResult;
}

impl<T : TaskExecutionListener + 'static> Listener for T {
    fn add_listener(self, freight: &mut FreightInner) {
        freight.add_task_execution_listener(self)
    }
}