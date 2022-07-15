//! The api defines the traits that assemble-daemon uses

#[macro_use]
extern crate static_assertions;

#[macro_use]
extern crate serde;

use crate::dependencies::Source;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::marker::PhantomData;

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

pub use defaults::task::DefaultTask;
pub use exception::BuildResult;
pub use project::Project;
pub use task::{Executable, Task};
pub use workspace::{default_workspaces::ASSEMBLE_HOME, Workspace};

pub mod prelude {
    pub use super::*;
    pub use properties::{Provides, ProvidesExt};
}

#[cfg(feature = "derive")]
pub use assemble_macros::*;

mod private {
    use crate::DefaultTask;

    /// Trait can only be implemented in the assemble core library.
    pub trait Sealed {}

    impl Sealed for DefaultTask {}
}

#[doc(hidden)]
pub mod __export {
    pub use crate::identifier::TaskId;
    pub use crate::properties::task_properties::TaskProperties;
    pub use crate::properties::{FromProperties, Provides, ProvidesExt};
    pub use crate::task::Executable;
}
