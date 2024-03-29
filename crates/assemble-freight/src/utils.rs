//! Utilities for fright to use

use crate::core::ConstructionError;
use assemble_core::error::PayloadError;
use assemble_core::identifier::{InvalidId, TaskId};
use assemble_core::project::error::ProjectError;
use assemble_core::task::flags::OptionsDecoderError;
use assemble_core::task::TaskOutcome;
use assemble_core::{BuildResult, payload_from, Project};

use log::SetLoggerError;

use std::fmt::{Debug, Formatter};
use std::io;
use std::marker::PhantomData;

use std::time::{Duration, Instant};
use thiserror::Error;

/// Represents the result of a task
pub struct TaskResult {
    /// The identifier of the task
    pub id: TaskId,
    /// The result of the task
    pub result: BuildResult,
    pub outcome: TaskOutcome,
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

    pub fn finish(self, result: BuildResult<TaskOutcome>) -> TaskResult {
        let duration = self.load_time.elapsed();
        let outcome = match &result {
            Ok(outcome) => outcome.clone(),
            Err(_) => TaskOutcome::Failed,
        };
        TaskResult {
            id: self.id,
            result: result.map(|_| ()),
            outcome,
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
    DecoderError(#[from] OptionsDecoderError),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error(transparent)]
    ConstructError(#[from] ConstructionError),
    #[error(transparent)]
    InvalidId(#[from] InvalidId),
    #[error(transparent)]
    SetLoggerError(#[from] SetLoggerError),
    #[error(transparent)]
    ClapError(#[from] clap::Error),
}



pub type FreightResult<T> = Result<T, PayloadError<FreightError>>;
