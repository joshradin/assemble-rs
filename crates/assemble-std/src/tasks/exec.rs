//! Contains code for the exec task.

use crate::specs::exec_spec::ExecSpec;
use crate::ProjectExec;
use assemble_core::exception::BuildException;
use assemble_core::{BuildResult, Project, Task};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;

/// The exec task runs a generic program using the built-in command runner of the OS
#[derive(Debug, Default)]
pub struct Exec {
    /// The exec spec of the task
    pub spec: ExecSpec,
}

/// Returned when the execution returns a non-zero exit code.
#[derive(Debug, thiserror::Error)]
#[error("Execution returned with non-zero exit code.")]
pub struct ExecError;
