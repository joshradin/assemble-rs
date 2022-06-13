//! Contains code for the exec task.

use crate::specs::exec_spec::ExecSpec;
use assemble_core::task::DynamicTaskAction;
use assemble_core::{BuildResult, Task, Project};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;
use assemble_core::exception::BuildException;
use crate::ProjectExec;

/// The exec task runs a generic program using the built-in command runner of the OS
#[derive(Debug, Default, Task)]
pub struct Exec {
    /// The exec spec of the task
    #[input]
    pub spec: ExecSpec,
}

/// Returned when the execution returns a non-zero exit code.
#[derive(Debug, thiserror::Error)]
#[error("Execution returned with non-zero exit code.")]
pub struct ExecError;

impl DynamicTaskAction for Exec {
    fn exec(&mut self, project: &Project) -> BuildResult {
        let status = project.exec_spec(self.spec.clone())?;
        if status.success() {
            Ok(())
        } else {
            Err(BuildException::new(ExecError))
        }
    }
}
