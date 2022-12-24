//! Core parts of freight

use assemble_core::error::PayloadError;
use assemble_core::identifier::TaskId;

use assemble_core::project::error::ProjectError;

mod task_resolver;
pub use task_resolver::*;

mod execution_plan;

pub use execution_plan::*;

#[derive(Debug, thiserror::Error)]
pub enum ConstructionError {
    #[error("No task named {0} found in project")]
    IdentifierNotFound(TaskId),
    #[error("Cycle found in between tasks {}", cycle.iter().map(ToString::to_string).collect::<Vec<_>>().join(","))]
    CycleFound { cycle: Vec<TaskId> },
    #[error(transparent)]
    ProjectError(#[from] ProjectError),
}
