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

#[cfg_attr(not(feature = "internal"), doc(hidden))]
pub mod internal;

pub mod logging;
pub mod plugins;
pub mod project;
pub mod resources;
pub mod task;
pub mod utilities;
pub mod web;
pub mod workflow;
pub mod workspace;

pub use defaults::task::DefaultTask;
pub use exception::BuildResult;
pub use project::Project;
pub use task::{property::TaskProperties, ExecutableTask, Task};
pub use workspace::{default_workspaces::ASSEMBLE_HOME, Workspace};

#[cfg(feature = "derive")]
pub use assemble_macros::*;

mod private {
    use crate::DefaultTask;

    /// Trait can only be implemented in the assemble core library.
    pub trait Sealed {}

    impl Sealed for DefaultTask {}
}
