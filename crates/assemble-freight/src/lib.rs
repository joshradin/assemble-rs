#![deny(rustdoc::broken_intra_doc_links)]

//! Freight is the main implementation library for how assemble projects are built.
//!
//! Binaries produced by the bin maker should use this library for execution purposes.

use std::sync::{Arc, RwLock};
use crate::core::{ExecutionGraph, ExecutionPlan, TaskResolver};
use assemble_core::plugins::{PluginAware, PluginManager};
use assemble_core::prelude::SharedProject;
use assemble_core::project::ProjectError;
use assemble_core::version::{Version, version};

use crate::project_properties::ProjectProperties;
use crate::utils::{FreightError, FreightResult, TaskResult, TaskResultBuilder};

#[macro_use]
extern crate log;

pub mod cli;
pub mod core;
pub mod ops;
pub mod project_properties;
pub mod utils;

pub use crate::cli::FreightArgs;