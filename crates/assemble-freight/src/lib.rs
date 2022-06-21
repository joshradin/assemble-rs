//! Freight is the main implementation library for how assemble projects are built.
//!
//! Binaries produced by the bin maker should use this library for execution purposes.

use std::num::NonZeroUsize;
use assemble_core::{BuildResult, ExecutableTask, Project};

use crate::core::{init_executor, TaskResolver};
use crate::utils::{FreightResult, TaskResult};
use assemble_core::logging::LoggingArgs;
use assemble_core::task::task_executor::TaskExecutor;
use clap::{Args, Parser};

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

pub mod core;
pub mod ops;
pub mod utils;

/// The main entry point into freight.
pub fn freight_main<E: ExecutableTask>(
    mut project: Project<E>,
    args: FreightArgs,
) -> FreightResult<Vec<TaskResult>> {
    args.log_level.init_root_logger();

    let mut resolver = TaskResolver::new(&mut project);

    let executor = init_executor(args.workers)?;
    let results = vec![];

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
        assert!(FreightArgs::try_parse_from(&["", "--workers","12", "--no-parallel"]).is_err());
    }
}
