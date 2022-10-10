#![deny(rustdoc::broken_intra_doc_links)]

//! Freight is the main implementation library for how assemble projects are built.
//!
//! Binaries produced by the bin maker should use this library for execution purposes.

use std::sync::{Arc, RwLock};
use crate::core::{ExecutionPlan, TaskResolver};
use assemble_core::plugins::{PluginAware, PluginManager};
use assemble_core::prelude::{Assemble, SharedProject, StartParameter};
use assemble_core::project::ProjectError;
use assemble_core::startup_api::execution_graph::ExecutionGraph;
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

pub mod startup;

pub use crate::cli::FreightArgs;

/// initializes the assemble instance
pub fn init_assemble<S : Into<StartParameter>>(args: S) -> FreightResult<Assemble> {
    let start_parameter = args.into();
    let assemble = Assemble::new(start_parameter);
    Ok(assemble)
}

/// Initializes assemble from the environment
pub fn init_assemble_from_env() -> FreightResult<Assemble> {
    let freight_args = FreightArgs::from_env();
    init_assemble(freight_args)
}