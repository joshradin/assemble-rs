//! The api defines the traits that assemble uses

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

#[cfg(feature = "internal")]
pub mod internal;
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
pub use task::{property::TaskProperties, IntoTask, Task};

#[cfg(feature = "derive")]
pub use assemble_macros::*;
