use crate::{FreightError, ProjectProperties};
use assemble_core::defaults::tasks::TaskReport;
use assemble_core::identifier::TaskId;
use assemble_core::logging::LoggingArgs;
use assemble_core::project::requests::TaskRequests;
use assemble_core::project::{ProjectResult, SharedProject};
use assemble_core::task::flags::{OptionRequest, WeakOptionsDecoder};
use clap::Parser;
use indexmap::IndexMap;
use indicatif::{ProgressState, ProgressStyle};
use std::collections::{BTreeMap, HashMap};
use std::env::args;
use std::io::Write;
use std::num::NonZeroUsize;
use std::str::FromStr;
use assemble_core::prelude::ProjectError;

/// The args to run Freight
#[derive(Debug, Parser)]
#[clap(about)]
#[clap(allow_hyphen_values = true)]
pub struct FreightArgs {
    /// Tasks to be run
    bare_task_requests: Vec<String>,
    /// Project properties. Set using -P or --prop
    #[clap(flatten)]
    pub properties: ProjectProperties,
    /// Log level to run freight in.
    #[clap(flatten)]
    pub logging: LoggingArgs,
    /// The number of workers to use.
    ///
    /// Defaults to the number of cpus on the host.
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

    /// Create a freight args instance from the surrounding environment.
    pub fn from_env() -> Self {
        Parser::parse()
    }

    /// Generate a task requests value using a shared project
    pub fn task_requests(&self, project: &SharedProject) -> ProjectResult<TaskRequests> {
        TaskRequests::build(project, &self.bare_task_requests)
    }
}


impl<S: AsRef<str>> FromIterator<S> for FreightArgs {
    fn from_iter<T: IntoIterator<Item = S>>(iter: T) -> Self {
        let mut args = vec![String::new()];
        args.extend(iter.into_iter().map(|s: S| s.as_ref().to_string()));

        FreightArgs::parse_from(args)
    }
}

pub fn main_progress_bar_style(failing: bool) -> ProgressStyle {
    let template = if failing {
        "{msg:>12.cyan.bold} [{bar:25.red.bright} {percent:>3}% ({pos}/{len})]  elapsed: {elapsed}"
    } else {
        "{msg:>12.cyan.bold} [{bar:25.green.bright} {percent:>3}% ({pos}/{len})]  elapsed: {elapsed}"
    };
    ProgressStyle::with_template(template)
        .unwrap()
        .progress_chars("=> ")
}
