#![deny(rustdoc::broken_intra_doc_links)]

//! Freight is the main implementation library for how assemble projects are built.
//!
//! Binaries produced by the bin maker should use this library for execution purposes.

use assemble_core::BuildResult;
use std::collections::HashSet;
use std::fmt::Debug;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use crate::core::TaskResolver;
use crate::ops::try_creating_plan;
use crate::project_properties::ProjectProperties;
use crate::utils::{FreightError, FreightResult, TaskResult, TaskResultBuilder};
use assemble_core::identifier::InvalidId;
use assemble_core::logging::LoggingArgs;
use assemble_core::project::{Project, ProjectError, SharedProject};
use assemble_core::task::task_executor::TaskExecutor;
use clap::{Args, Parser};
use colored::Colorize;
use log::{Level, LevelFilter};
use ops::init_executor;

#[macro_use]
extern crate log;

pub mod core;
pub mod ops;
pub mod project_properties;
pub mod utils;

pub use crate::core::cli::FreightArgs;

#[cfg(test)]
mod test {
    use super::*;
    use crate::core::cli::FreightArgs;

    #[test]
    fn no_parallel() {
        let args: FreightArgs = FreightArgs::parse_from(&["", "--no-parallel"]);
        println!("{:#?}", args);
        assert!(args.no_parallel);
        assert_eq!(args.workers.get(), 1);
    }

    #[test]
    fn arbitrary_workers() {
        let args: FreightArgs = FreightArgs::parse_from(&["", "--workers", "13"]);
        println!("{:#?}", args);
        assert_eq!(args.workers.get(), 13);
        assert!(FreightArgs::try_parse_from(&["", "-J", "0"]).is_err());
    }

    #[test]
    fn default_workers_is_num_cpus() {
        let args: FreightArgs = FreightArgs::parse_from(&[""]);
        assert_eq!(args.workers.get(), num_cpus::get());
    }

    #[test]
    fn workers_and_no_parallel_conflicts() {
        assert!(FreightArgs::try_parse_from(&["", "--workers", "12", "--no-parallel"]).is_err());
    }
}
