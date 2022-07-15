#![deny(broken_intra_doc_links)]

//! Freight is the main implementation library for how assemble projects are built.
//!
//! Binaries produced by the bin maker should use this library for execution purposes.

use assemble_core::{BuildResult, Executable};
use std::collections::HashSet;
use std::fmt::Debug;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use crate::core::TaskResolver;
use crate::ops::try_creating_plan;
use crate::utils::{FreightError, FreightResult, TaskResult, TaskResultBuilder};
use assemble_core::identifier::InvalidId;
use assemble_core::logging::LoggingArgs;
use assemble_core::project::{Project, ProjectError};
use assemble_core::task::task_executor::TaskExecutor;
use clap::{Args, Parser};
use ops::init_executor;

/// The args to run Freight
#[derive(Debug, Parser)]
#[clap(about)]
pub struct FreightArgs {
    /// Tasks to be run
    pub tasks: Vec<String>,
    /// Log level to run freight in.
    #[clap(flatten)]
    pub log_level: LoggingArgs,
    /// The number of workers to use. Defaults to the number of cpus on the host.
    #[clap(long, short = 'J')]
    #[clap(default_value_t = NonZeroUsize::new(num_cpus::get()).expect("Number of cpus should never be 0"))]
    #[clap(default_value_if("no-parallel", None, Some("1")))]
    pub workers: NonZeroUsize,
    /// Don't run with parallel tasks
    #[clap(long)]
    #[clap(conflicts_with = "workers")]
    pub no_parallel: bool,
}

impl FreightArgs {
    /// Simulate creating the freight args from the command line
    pub fn command_line<S: AsRef<str>>(cmd: S) -> Self {
        <Self as FromIterator<_>>::from_iter(cmd.as_ref().split_whitespace())
    }
}

impl<S: AsRef<str>> FromIterator<S> for FreightArgs {
    fn from_iter<T: IntoIterator<Item = S>>(iter: T) -> Self {
        let mut args = vec![String::new()];
        args.extend(iter.into_iter().map(|s: S| s.as_ref().to_string()));

        FreightArgs::parse_from(args)
    }
}

pub mod core;
pub mod ops;
pub mod utils;

/// The main entry point into freight.
pub fn freight_main(mut project: Project, args: FreightArgs) -> FreightResult<Vec<TaskResult>> {
    args.log_level.init_root_logger();

    let mut resolver = TaskResolver::new(&mut project);
    let requests = args
        .tasks
        .into_iter()
        .map(|t| {
            resolver
                .try_find_identifier(&t)
                .ok_or(FreightError::ProjectError(ProjectError::InvalidIdentifier(
                    InvalidId(t),
                )))
        })
        .collect::<Result<Vec<_>, _>>()?;

    println!("Attempting to create exec graph...");
    let exec_graph = resolver.to_execution_graph(&requests)?;
    println!("created exec graph: {:?}", exec_graph);
    let mut exec_plan = try_creating_plan(exec_graph)?;

    println!("exec plan: {:#?}", exec_plan);

    let executor = init_executor(args.workers)?;

    let mut results = vec![];

    // let mut work_queue = TaskExecutor::new(project, &executor);

    while !exec_plan.finished() {
        if let Some(mut task) = exec_plan.pop_task() {
            let result_builder = TaskResultBuilder::new(&task);
            let output = task.execute(&project);
            exec_plan.report_task_status(task.task_id(), output.is_ok());
            let work_result = result_builder.finish(output);
            results.push(work_result);
        }
    }

    // drop(work_queue);
    executor.join()?; // force the executor to terminate safely.
    Ok(results)
}

#[cfg(test)]
mod test {
    use super::*;

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
