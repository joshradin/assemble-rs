//! Contains code for the exec task.

use crate::specs::exec_spec::ExecSpec;
use assemble_core::task::DynamicTaskAction;
use assemble_core::{BuildResult, IntoTask, Project};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;

/// The exec task runs a generic program using the built-in command runner of the OS
#[derive(Debug, Default, IntoTask)]
pub struct Exec {
    /// The exec spec of the task
    #[input]
    pub spec: ExecSpec,
}

impl DynamicTaskAction for Exec {
    fn exec(&mut self, project: &Project) -> BuildResult {
        todo!()
    }
}
