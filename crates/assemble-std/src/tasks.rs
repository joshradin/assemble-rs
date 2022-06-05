//! The default tasks

use assemble_core::{BuildResult, IntoTask, Project, Task};

pub mod exec;
pub mod files;

/// A task that has no actions by default
#[derive(Debug, Default, IntoTask)]
#[action(no_action)]
pub struct Empty;

/// A no-op task action
pub fn no_action(_: &dyn Task, _: &Project) -> BuildResult {
    Ok(())
}
