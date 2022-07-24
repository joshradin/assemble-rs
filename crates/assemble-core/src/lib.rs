//! The api defines the traits that assemble-daemon uses

#[macro_use]
extern crate static_assertions;

#[macro_use]
extern crate serde;

pub mod assemble;
pub mod defaults;
pub mod dependencies;
pub mod exception;
pub mod file;
pub mod file_collection;
pub mod fingerprint;
pub mod flow;
pub mod identifier;
pub mod immutable;
#[cfg_attr(not(feature = "internal"), doc(hidden))]
pub mod internal;
pub mod logging;
pub mod named;
pub mod plugins;
pub mod project;
pub mod properties;
pub mod resources;
pub mod task;
pub mod utilities;

pub mod web;
pub mod work_queue;
pub mod workflow;
pub mod workspace;

pub use exception::BuildResult;
pub use project::Project;
use std::fmt::Display;
pub use task::Task;
pub use workspace::{default_workspaces::ASSEMBLE_HOME, Workspace};

pub mod prelude {
    pub use super::*;
    pub use properties::{Provides, ProvidesExt};
}

pub(crate) use utilities::ok;

#[cfg(feature = "derive")]
pub use assemble_macros::*;
pub use task::Executable;

mod private {

    /// Trait can only be implemented in the assemble core library.
    pub trait Sealed {}
}

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
    pub use crate::properties::{Provides, ProvidesExt};
    pub use crate::task::{CreateTask, Executable, InitializeTask};
}
