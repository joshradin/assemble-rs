//! Provides listeners

use crate::startup_api::invocation::Assemble;
use crate::task::{ExecutableTask, TaskOutcome};

use std::fmt::{Debug, Formatter};

use crate::prelude::*;
use crate::startup_api::execution_graph::ExecutionGraph;

/// A listener than can be added to a type.
pub trait Listener {
    type Listened;

    /// Add a listener to freight
    fn add_listener(self, freight: &mut Self::Listened) -> ProjectResult;
}

/// A listener that listens for task execution
pub trait TaskExecutionListener: Debug + Listener<Listened = Assemble> {
    /// Listens for tasks to finish executing
    fn after_execute(&mut self, task: &dyn ExecutableTask, outcome: TaskOutcome) -> ProjectResult;
    /// Listens for tasks that are about to start executing
    fn before_execute(&mut self, task: &dyn ExecutableTask) -> ProjectResult;
}

/// Listens for the task execution graph to be ready
pub trait TaskExecutionGraphListener: Debug + Listener<Listened = Assemble> {
    fn graph_ready(&mut self, graph: &ExecutionGraph) -> ProjectResult;
}

/// A listener for when the graph is ready
pub struct GraphReady {
    function: Box<dyn FnMut(&ExecutionGraph) -> ProjectResult>,
}

impl GraphReady {
    pub fn new<F: FnMut(&ExecutionGraph) -> ProjectResult + 'static>(func: F) -> Self {
        Self {
            function: Box::new(func),
        }
    }
}

impl Debug for GraphReady {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphReady").finish_non_exhaustive()
    }
}

impl Listener for GraphReady {
    type Listened = Assemble;

    fn add_listener(self, freight: &mut Self::Listened) -> ProjectResult {
        freight.add_task_execution_graph_listener(self)
    }
}

impl TaskExecutionGraphListener for GraphReady {
    fn graph_ready(&mut self, graph: &ExecutionGraph) -> ProjectResult {
        (self.function)(graph)
    }
}
