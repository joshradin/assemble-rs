//! Core parts of freight

use std::io;
use std::num::NonZeroUsize;
use assemble_core::{ExecutableTask, Project};
use assemble_core::project::ProjectError;
use assemble_core::task::TaskIdentifier;
use assemble_core::work_queue::WorkerExecutor;

mod task_resolver;
pub use task_resolver::*;

mod task_order;
pub use task_order::*;
use crate::FreightError;

#[derive(Debug, thiserror::Error)]
pub enum ConstructionError {
    #[error("No task named {0} found in project")]
    IdentifierNotFound(TaskIdentifier),
    #[error("Cycle found in between tasks {}", cycle.into_iter().map(ToString::to_string).collect::<Vec<_>>().join(","))]
    CycleFound { cycle: Vec<TaskIdentifier> },
    #[error(transparent)]
    ProjectError(#[from] ProjectError),

}


