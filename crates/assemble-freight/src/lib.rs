#![deny(rustdoc::broken_intra_doc_links)]

//! Freight is the main implementation library for how assemble projects are built.
//!
//! Binaries produced by the bin maker should use this library for execution purposes.

use assemble_core::BuildResult;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use crate::core::TaskResolver;
use crate::ops::try_creating_plan;
use crate::project_properties::ProjectProperties;
use crate::utils::{FreightError, FreightResult, TaskResult, TaskResultBuilder};
use assemble_core::identifier::InvalidId;
use assemble_core::logging::LoggingArgs;
use assemble_core::project::error::ProjectError;
use assemble_core::project::{Project, SharedProject};
use assemble_core::task::task_executor::TaskExecutor;
use clap::{Args, Parser};
use colored::Colorize;
use log::{Level, LevelFilter};
use ops::init_executor;

#[macro_use]
extern crate log;

pub mod cli;
pub mod core;
pub mod ops;
pub mod project_properties;
pub mod utils;

pub use crate::cli::FreightArgs;
