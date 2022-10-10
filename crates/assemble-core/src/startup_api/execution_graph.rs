use crate::__export::TaskIO;
use crate::project::requests::TaskRequests;
use crate::task::{FullTask, TaskOrderingKind};
use parking_lot::RwLock;
use petgraph::graph::DiGraph;
use std::sync::Arc;

/// The Execution Plan provides a plan of executable tasks that
/// the task executor can execute.
///
/// For the execution plan to be valid, the following must hold:
/// - No Cycles
/// - The graph must be able to be topographically sorted such that all tasks that depend on a task
///     run before a task, and all tasks that finalize a task occur after said task
#[derive(Debug, Clone)]
pub struct ExecutionGraph {
    /// The task ordering graph
    graph: Arc<RwLock<DiGraph<SharedAnyTask, TaskOrderingKind>>>,
    /// Tasks requested
    requested_tasks: Arc<TaskRequests>,
}

impl ExecutionGraph {
    pub fn new(graph: DiGraph<SharedAnyTask, TaskOrderingKind>, requested_tasks: TaskRequests) -> Self {
        Self {
            graph: Arc::new(RwLock::new(graph)),
            requested_tasks: Arc::new(requested_tasks),
        }
    }
    pub fn requested_tasks(&self) -> &Arc<TaskRequests> {
        &self.requested_tasks
    }
    pub fn graph(&self) -> &Arc<RwLock<DiGraph<SharedAnyTask, TaskOrderingKind>>> {
        &self.graph
    }
}

pub type SharedAnyTask = Arc<RwLock<Box<dyn FullTask>>>;