//! Freight is the main implementation library for how assemble projects are built.
//!
//! Binaries produced by the bin maker should use this library for execution purposes.

use assemble_core::{BuildResult, ExecutableTask, Project};

use clap::Args;
use assemble_core::logging::LoggingArgs;
use crate::utils::{FreightResult, TaskResult};

#[derive(Debug, Args)]
#[clap(about)]
pub struct FreightArgs {
    /// Tasks to be run
    pub tasks: Vec<String>,
    /// Log level to run freight in.
    #[clap(flatten)]
    pub log_level: LoggingArgs,
    /// The number of workers to use. Defaults to the number of cpus on the host.
    #[clap(long)]
    #[clap(default_value_t = num_cpus::get())]
    pub workers: usize
}

pub mod core;
pub mod ops;
pub mod utils;

/// The main entry point into freight.
pub fn freight_main<E : ExecutableTask>(project: Project<E>, args: FreightArgs) -> FreightResult<Vec<TaskResult>> {
    args.log_level.init_logger();



    let results = vec![];

    Ok(results)
}





