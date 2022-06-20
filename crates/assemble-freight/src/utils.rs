//! Utilities for fright to use

use std::io;
use std::marker::PhantomData;
use std::time::{Duration, Instant};
use assemble_core::BuildResult;
use assemble_core::task::TaskIdentifier;
use thiserror::Error;
use assemble_core::project::ProjectError;

/// Represents the result of a task
pub struct TaskResult {
    /// The identifier of the task
    pub id: TaskIdentifier,
    /// The result of the task
    pub result: BuildResult,
    /// The time the task was loaded into the executor
    pub load_time: Instant,
    /// The duration between the load time and when a result was received
    pub duration: Duration,
    /// The stdout of the task
    pub stdout: Vec<u8>,
    /// The stderr of the task
    pub stderr: Vec<u8>,
    /// Prevent construction
    _data: PhantomData<()>
}

/// An error occurred while freight was running
#[derive(Debug, Error)]
pub enum FreightError {
    #[error(transparent)]
    ProjectError(#[from] ProjectError),
    #[error(transparent)]
    IoError(#[from] io::Error)
}

pub type FreightResult<T> = Result<T, FreightError>;

