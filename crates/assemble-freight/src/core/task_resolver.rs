use petgraph::prelude::*;
use assemble_core::{ExecutableTask, Project};
use assemble_core::task::TaskOrdering;

pub struct TaskResolver<'proj, T : ExecutableTask> {
    project: &'proj mut Project<T>
}

impl<'proj, T: ExecutableTask> TaskResolver<'proj, T> {
    pub fn new(project: &'proj mut Project<T>) -> Self {
        Self { project }
    }
}

/// The Execution Graph provides a graph of executable tasks that
/// the task executor can execute.
pub struct ExecutionGraph<E : ExecutableTask> {
    graph: DiGraph<E, TaskOrdering>
}