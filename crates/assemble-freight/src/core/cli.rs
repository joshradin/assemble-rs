use crate::{FreightError, ProjectProperties};
use assemble_core::defaults::tasks::TaskReport;
use assemble_core::identifier::TaskId;
use assemble_core::logging::LoggingArgs;
use assemble_core::project::error::{ProjectError, ProjectResult};
use assemble_core::project::requests::TaskRequests;
use assemble_core::project::SharedProject;
use assemble_core::task::flags::{OptionRequest, WeakOptionsDecoder};
use clap::builder::ArgPredicate;
use clap::Parser;
use indexmap::IndexMap;
use indicatif::{ProgressState, ProgressStyle};
use std::collections::{BTreeMap, HashMap};
use std::env::args;
use std::io::Write;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::str::FromStr;

/// Command line options for running assemble based projects.
///
/// Tasks can either be the full path for the task, or a relative one from the directory
/// in use within the project.
///
/// Task options are configured on per task basis and are fully configured at
/// compile time. Options for tasks must immediately follow the task request.
///
/// When many tasks are matched for the same task request, they all
/// receive the same task options.
#[derive(Debug, Parser, Clone)]
#[clap(name = "assemble")]
#[clap(version, author)]
#[clap(before_help = format!("{} v{}", clap::crate_name!(), clap::crate_version!()))]
#[clap(after_help = "For project specific information, use the :help task.")]
#[clap(allow_hyphen_values = true)]
#[clap(term_width = 64)]
pub struct FreightArgs {
    /// Tasks to be run
    #[clap(value_name = "TASK [TASK OPTIONS]...")]
    bare_task_requests: Vec<String>,
    /// Project lazy_evaluation. Set using -P or --prop
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
    #[clap(default_value_if("no_parallel", ArgPredicate::IsPresent, "1"))]
    #[clap(help_heading = None)]
    pub workers: NonZeroUsize,
    /// Don't run with parallel tasks
    #[clap(long)]
    #[clap(conflicts_with = "workers")]
    #[clap(help_heading = None)]
    pub no_parallel: bool,

    /// Use an alternative settings file
    #[clap(short = 'F')]
    #[clap(help_heading = None)]
    pub settings_file: Option<PathBuf>,


    /// Display backtraces for errors if possible.
    #[clap(short = 'B', long)]
    #[clap(help_heading = None)]
    pub backtrace: bool,

    /// Forces all tasks to be rerun
    #[clap(long)]
    #[clap(help_heading = None)]
    pub rerun_tasks: bool,
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

    /// Creates a clone with different tasks requests
    pub fn with_tasks<'s, I: IntoIterator<Item = &'s str>>(&self, iter: I) -> FreightArgs {
        let mut clone = self.clone();
        clone.bare_task_requests = iter.into_iter().map(|s| s.to_string()).collect();
        clone
    }

    /// Gets a property.
    pub fn property(&self, key: impl AsRef<str>) -> Option<&str> {
        self.properties.property(key)
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::core::cli::FreightArgs;

    #[test]
    fn no_parallel() {
        let args: FreightArgs = FreightArgs::command_line("--no-parallel");
        println!("{:#?}", args);
        assert!(args.no_parallel);
        assert_eq!(args.workers.get(), 1);
    }

    #[test]
    fn arbitrary_workers() {
        let args: FreightArgs = FreightArgs::command_line("--workers 13");
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

    #[test]
    fn can_set_project_properties() {
        let args = FreightArgs::command_line("-P hello=world -P key1 -P key2");
        assert_eq!(args.property("hello"), Some("world"));
        assert_eq!(args.property("key1"), Some(""));
        assert_eq!(args.property("key2"), Some(""));
    }


}
