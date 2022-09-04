//! The api defines the traits that assemble-daemon uses

#[macro_use]
extern crate static_assertions;

#[macro_use]
extern crate serde;

#[macro_use]
extern crate log;

pub mod assemble;
pub mod cache;
pub mod cryptography;
pub mod defaults;
pub mod dependencies;
pub mod exception;
pub mod file;
pub mod file_collection;
pub mod fingerprint;
pub mod flow;
pub mod identifier;
pub mod immutable;
pub mod logging;
pub mod named;
pub mod plugins;
pub mod project;
pub mod properties;
pub mod resources;
pub mod task;
pub(crate) mod unstable;
pub mod utilities;
pub mod web;
pub mod work_queue;
pub mod workflow;
pub mod workspace;
pub mod version;

// Re-exports
pub use exception::BuildResult;
pub use project::Project;
pub use task::Executable;
pub use task::Task;
#[cfg(feature = "unstable")]
pub use unstable::enabled::*;
pub(crate) use unstable::*;
pub use workspace::{default_workspaces::ASSEMBLE_HOME, Workspace};

pub mod prelude {
    //! Provides many useful, often use types and functions within assemble

    pub use super::*;
    pub use project::{ProjectError, ProjectResult, SharedProject};
    pub use properties::{Provides, ProvidesExt};
    #[cfg(feature = "unstable")]
    pub use unstable::enabled::prelude::*;

    pub use identifier::{ProjectId, TaskId};
}

pub(crate) use utilities::ok;

#[cfg(feature = "derive")]
pub use assemble_macros::*;

mod private {

    /// Trait can only be implemented in the assemble core library.
    pub trait Sealed {}
}

use std::fmt::Display;

/// Executes some function. If an error is returned by the function, then `None` is returned and
/// the error is printed to the error output. Otherwise, `Some(R)` is returned.
pub fn execute_assemble<R, E, F>(func: F) -> Option<R>
where
    E: Display,
    F: FnOnce() -> Result<R, E>,
{
    match func() {
        Ok(o) => Some(o),
        Err(e) => {
            eprintln!("error: {}", e);
            None
        }
    }
}

#[doc(hidden)]
pub mod __export {
    pub use crate::identifier::TaskId;
    pub use crate::project::{Project, ProjectError, ProjectResult};
    pub use crate::properties::{Provides, ProvidesExt};
    pub use crate::task::{CreateTask, Executable, InitializeTask, TaskIO};
}
