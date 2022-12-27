//! # `assemble-core`
//!
//! The api defines the structs, functions, and traits that make up the assemble project.
//!
//!

#![deny(rustdoc::broken_intra_doc_links)]

#[macro_use]
extern crate static_assertions;

#[macro_use]
extern crate serde;

#[macro_use]
extern crate log;

pub mod cache;
pub mod cargo;
pub mod cryptography;
pub mod defaults;
pub mod dependencies;
pub mod error;
pub mod exception;
pub mod file;
pub mod file_collection;
pub mod fingerprint;
pub mod flow;
pub mod identifier;
pub mod immutable;
pub mod lazy_evaluation;
pub mod logging;
pub mod named;
pub mod plugins;
pub mod project;
pub mod resources;
pub mod startup;
pub mod task;
pub(crate) mod unstable;
pub mod utilities;
pub mod version;
pub mod web;
pub mod work_queue;
pub mod workflow;
pub mod workspace;

// Re-exports
pub use exception::BuildResult;
pub use plugins::Plugin;
pub use project::Project;
pub use task::Executable;
pub use task::Task;
#[cfg(feature = "unstable")]
pub use unstable::enabled::*;

pub use workspace::{default_workspaces::ASSEMBLE_HOME, Workspace};

pub mod prelude {
    //! Provides many useful, often use types and functions within assemble

    pub use super::*;
    pub use crate::project::shared::SharedProject;
    pub use lazy_evaluation::{Provider, ProviderExt};
    pub use plugins::{Plugin, PluginAware, PluginManager};
    #[cfg(feature = "unstable")]
    pub use unstable::enabled::prelude::*;

    pub use startup::{initialization::*, invocation::*, listeners};

    pub use crate::error::Result;
    pub use crate::project::error::ProjectError;
    pub use crate::project::error::ProjectResult;
    pub use identifier::{ProjectId, TaskId};

    pub use std::result::Result as StdResult;
}

pub(crate) use utilities::ok;

#[cfg(feature = "derive")]
pub use assemble_macros::*;

mod private {
    use crate::prelude::{Assemble, Settings};
    use parking_lot::RwLock;
    use std::sync::Arc;

    /// Trait can only be implemented in the assemble core library.
    pub trait Sealed {}

    impl Sealed for Arc<RwLock<Settings>> {}
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
    pub use crate::lazy_evaluation::{Provider, ProviderExt};
    pub use crate::project::error::ProjectError;
    pub use crate::project::error::ProjectResult;
    pub use crate::project::Project;
    pub use crate::task::create_task::CreateTask;
    pub use crate::task::initialize_task::InitializeTask;
    pub use crate::task::task_io::TaskIO;
    pub use crate::task::{work_handler::serializer::*, Executable};
}
