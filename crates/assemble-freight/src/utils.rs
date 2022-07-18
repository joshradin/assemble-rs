//! Utilities for fright to use

use crate::core::ConstructionError;
use assemble_core::identifier::{InvalidId, TaskId};
use assemble_core::project::ProjectError;
use assemble_core::task::HasTaskId;
use assemble_core::{BuildResult, Task};
use backtrace::Backtrace;
use std::error::Error;
use std::fmt::{Debug, Formatter};
use std::io;
use std::marker::PhantomData;
use std::num::{IntErrorKind, ParseIntError};
use std::time::{Duration, Instant};
use thiserror::Error;

/// Represents the result of a task
pub struct TaskResult {
    /// The identifier of the task
    pub id: TaskId,
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
    _data: PhantomData<()>,
}

impl Debug for TaskResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -> {:?}", self.id, self.result)
    }
}

pub struct TaskResultBuilder {
    id: TaskId,
    load_time: Instant,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl TaskResultBuilder {
    pub fn new(task: TaskId) -> Self {
        Self {
            id: task,
            load_time: Instant::now(),
            stdout: vec![],
            stderr: vec![],
        }
    }

    pub fn finish(self, result: BuildResult) -> TaskResult {
        let duration = self.load_time.elapsed();
        TaskResult {
            id: self.id,
            result,
            load_time: self.load_time,
            duration,
            stdout: self.stdout,
            stderr: self.stderr,
            _data: Default::default(),
        }
    }
}

/// An error occurred while freight was running
#[derive(Debug, Error)]
pub enum FreightError {
    #[error(transparent)]
    ProjectError(#[from] ProjectError),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error(transparent)]
    ConstructError(#[from] ConstructionError),
    #[error(transparent)]
    InvalidId(#[from] InvalidId),
}

pub type FreightResult<T> = Result<T, FreightError>;
