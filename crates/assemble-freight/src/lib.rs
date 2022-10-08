#![deny(rustdoc::broken_intra_doc_links)]

//! Freight is the main implementation library for how assemble projects are built.
//!
//! Binaries produced by the bin maker should use this library for execution purposes.

use assemble_core::version::Version;
use crate::core::{ExecutionGraph, ExecutionPlan, TaskResolver};

use crate::project_properties::ProjectProperties;
use crate::utils::{FreightResult, TaskResult, TaskResultBuilder};

#[macro_use]
extern crate log;

pub mod cli;
pub mod core;
pub mod ops;
pub mod project_properties;
pub mod utils;
pub mod listeners;

pub use crate::cli::FreightArgs;

/// new way to access freight
pub struct Freight {
    args: FreightArgs,
    task_plan: ExecutionGraph,
    version: Version,
}
