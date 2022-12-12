use std::ffi::OsString;

use std::path::{Path, PathBuf};

use clap::error::ErrorKind;
use clap::{Args, Command, CommandFactory, Error, FromArgMatches, Parser};

use indicatif::ProgressStyle;
use itertools::Itertools;
use merge::Merge;

use assemble_core::logging::LoggingArgs;
use assemble_core::prelude::BacktraceEmit;
use assemble_core::project::error::ProjectResult;
use assemble_core::project::requests::TaskRequests;
use assemble_core::project::SharedProject;

use crate::ProjectProperties;

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
#[derive(Debug, Parser, Clone, Merge)]
#[clap(name = "assemble")]
#[clap(version, author)]
#[clap(before_help = format!("{} v{}", clap::crate_name!(), clap::crate_version!()))]
#[clap(after_help = "For project specific information, use the :help task.")]
#[clap(term_width = 64)]
pub struct FreightArgs {
    /// Project lazy_evaluation. Set using -P or --prop
    #[clap(flatten)]
    properties: ProjectProperties,
    /// Log level to run freight in.
    #[clap(flatten)]
    logging: LoggingArgs,

    /// The number of workers to use.
    ///
    /// Defaults to the number of cpus on the host.
    #[clap(long, short = 'J')]
    #[clap(help_heading = None)]
    #[clap(value_parser = clap::value_parser!(u32).range(1..))]
    workers: Option<u32>,
    /// Don't run with parallel tasks
    #[clap(long)]
    #[clap(conflicts_with = "workers")]
    #[clap(help_heading = None)]
    #[merge(strategy = merge::bool::overwrite_false)]
    no_parallel: bool,

    /// Use an alternative settings file
    #[clap(short = 'F')]
    #[clap(help_heading = None)]
    settings_file: Option<PathBuf>,

    /// Display backtraces for errors if possible.
    #[clap(short = 'b', long)]
    #[clap(help_heading = None)]
    #[merge(strategy = merge::bool::overwrite_false)]
    backtrace: bool,

    /// Display backtraces for errors if possible.
    #[clap(short = 'B', long)]
    #[clap(help_heading = None)]
    #[merge(strategy = merge::bool::overwrite_false)]
    #[clap(conflicts_with = "backtrace")]
    long_backtrace: bool,

    /// Forces all tasks to be rerun
    #[clap(long)]
    #[clap(help_heading = None)]
    #[merge(strategy = merge::bool::overwrite_false)]
    rerun_tasks: bool,

    #[clap(flatten)]
    bare_task_requests: TaskRequestsArgs,
}

#[derive(Debug, Clone, Args, merge::Merge)]
struct TaskRequestsArgs {
    /// Request tasks to be executed by assemble
    #[clap(value_name = "TASK [TASK OPTIONS]...")]
    // #[clap(allow_hyphen_values = true)]
    #[clap(help_heading = "Tasks")]
    #[merge(strategy = merge::vec::append)]
    requests: Vec<String>,
}

impl<S: AsRef<str>> FromIterator<S> for TaskRequestsArgs {
    fn from_iter<T: IntoIterator<Item = S>>(iter: T) -> Self {
        Self {
            requests: iter.into_iter().map(|s| s.as_ref().to_string()).collect(),
        }
    }
}

impl TaskRequestsArgs {
    fn requests(&self) -> &Vec<String> {
        &self.requests
    }
}

impl FreightArgs {
    /// Simulate creating the freight args from the command line
    pub fn command_line<S: AsRef<str>>(cmd: S) -> Self {
        Self::try_command_line(cmd).expect("Couldn't parse cmd line")
    }

    /// Simulate creating the freight args from the command line
    pub fn try_command_line<S: AsRef<str>>(cmd: S) -> Result<Self, Error> {
        Self::try_parse(cmd.as_ref().split_whitespace())
    }

    /// Create a freight args instance from the surrounding environment.
    pub fn from_env() -> Self {
        match Self::try_parse(std::env::args_os().skip(1)) {
            Ok(s) => s,
            Err(e) => {
                e.exit();
            }
        }
    }

    fn try_parse<S, I: IntoIterator<Item = S>>(iter: I) -> Result<Self, clap::Error>
    where
        S: Into<OsString>,
    {
        let mut parsed_freight_args: FreightArgs = Parser::parse_from([""]);
        let empty = OsString::from("");

        let mut index = 0;
        let mut window_size = 1;

        let args: Vec<OsString> = iter.into_iter().map(|s: S| s.into()).collect();
        let mut last_error = None;

        let mut parsed_args = vec![&empty];

        while index + window_size <= args.len() {
            let mut arg_window = Vec::from_iter(&args[index..][..window_size]);
            arg_window.insert(0, &empty);

            let intermediate = <FreightArgs as Parser>::try_parse_from(&arg_window);

            match intermediate {
                Ok(arg_matches) => {
                    parsed_freight_args.merge(arg_matches);
                    parsed_args.extend(arg_window);

                    <FreightArgs as Parser>::try_parse_from(&parsed_args)?;
                    index += window_size;
                    window_size = 1;
                }
                Err(e) => {
                    last_error = if e.kind() == ErrorKind::UnknownArgument {
                        // add anyway, maybe a task command
                        if parsed_freight_args.bare_task_requests.requests.is_empty() {
                            Some(e);
                            break;
                        } else {
                            parsed_freight_args.bare_task_requests.requests.extend(
                                arg_window
                                    .drain(1..)
                                    .map(|s| s.to_str().unwrap().to_string()),
                            );
                            index += 1;
                            Some(e)
                        }
                    } else if e.kind() == ErrorKind::InvalidValue {
                        window_size += 1;
                        Some(e)
                    } else {
                        return Err(e);
                    }
                }
            }
        }

        if index == args.len() {
            Ok(parsed_freight_args)
        } else if let Some(e) = last_error {
            Err(e)
        } else {
            let mut command: Command = FreightArgs::command();
            Err(
                Error::raw(ErrorKind::UnknownArgument, "failed for unknown reason")
                    .format(&mut command),
            )
        }
    }

    /// Generate a task requests value using a shared project
    pub fn task_requests(&self, project: &SharedProject) -> ProjectResult<TaskRequests> {
        TaskRequests::build(project, self.bare_task_requests.requests())
    }

    /// Generate a task requests value using a shared project
    pub fn task_requests_raw(&self) -> &[String] {
        &self.bare_task_requests.requests[..]
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

    /// Gets the logging args
    pub fn logging(&self) -> &LoggingArgs {
        &self.logging
    }

    /// Gets the number of workers
    pub fn workers(&self) -> usize {
        if self.no_parallel {
            1
        } else {
            self.workers
                .map(|w| w as usize)
                .unwrap_or_else(num_cpus::get)
        }
    }

    /// Gets an optional, alternative settings file instead of the default one
    pub fn settings_file(&self) -> Option<&Path> {
        self.settings_file.as_deref()
    }

    /// Get whether to emit backtraces or not.
    pub fn backtrace(&self) -> BacktraceEmit {
        match (self.backtrace, self.long_backtrace) {
            (true, false) => {
                BacktraceEmit::Short
            }
            (_, true) => BacktraceEmit::Long,
            _ => BacktraceEmit::None
        }
    }

    /// Get whether to always rerun tasks.
    pub fn rerun_tasks(&self) -> bool {
        self.rerun_tasks
    }
    pub fn properties(&self) -> &ProjectProperties {
        &self.properties
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
    use assemble_core::logging::ConsoleMode;
    use clap::{Command, CommandFactory};
    use log::LevelFilter;

    use crate::cli::FreightArgs;

    #[test]
    fn can_render_help() {
        let mut freight_command: Command = FreightArgs::command();
        let str = freight_command.render_help();
        println!("{}", str);
    }

    #[test]
    fn no_parallel() {
        let args: FreightArgs = FreightArgs::command_line("--no-parallel");
        println!("{:#?}", args);
        assert!(args.no_parallel);
        assert_eq!(args.workers(), 1);
    }

    #[test]
    fn arbitrary_workers() {
        let args: FreightArgs = FreightArgs::command_line("--workers 13");
        println!("{:#?}", args);
        assert_eq!(args.workers(), 13);
    }

    #[test]
    fn default_workers_is_num_cpus() {
        let args: FreightArgs = FreightArgs::command_line("");
        assert_eq!(args.workers(), num_cpus::get());
    }

    #[test]
    fn zero_workers_illegal() {
        assert!(
            FreightArgs::try_command_line("-J 0").is_err(),
            "0 workers is illegal, but error wasn't properly detected"
        );
    }

    #[test]
    fn workers_and_no_parallel_conflicts() {
        assert!(FreightArgs::try_command_line("-J 2 --no-parallel").is_err());
    }

    #[test]
    fn can_set_project_properties() {
        let args = FreightArgs::command_line("-P hello=world -P key1 -P key2");
        assert_eq!(args.property("hello"), Some("world"));
        assert_eq!(args.property("key1"), Some(""));
        assert_eq!(args.property("key2"), Some(""));
    }

    #[test]
    fn arbitrary_task_positions() {
        let args = FreightArgs::try_command_line(":tasks --all --debug --workers 6 help");
        if args.is_err() {
            eprintln!("{}", args.unwrap_err());
            panic!("Couldn't parse");
        }
        let args = args.unwrap();
        println!("args: {:#?}", args);
        assert_eq!(
            args.logging.log_level_filter(),
            LevelFilter::Debug,
            "debug log level not set"
        );
        assert_eq!(args.workers(), 6, "should set 6 workers");
        assert_eq!(args.bare_task_requests.requests().len(), 3);
        assert_eq!(
            &args.bare_task_requests.requests()[..2],
            &[":tasks", "--all"],
            "first task request is tasks --all"
        );
        assert_eq!(
            args.bare_task_requests.requests()[2],
            "help",
            "second task request is help"
        );
    }

    #[test]
    fn tasks_last() {
        let args = FreightArgs::command_line("--debug --workers 6 -- :tasks --all help");
        println!("args: {:#?}", args);
        assert_eq!(
            args.logging.log_level_filter(),
            LevelFilter::Debug,
            "debug log level not set"
        );
        assert_eq!(args.workers(), 6, "should set 6 workers");
        assert_eq!(args.bare_task_requests.requests().len(), 3);
        assert_eq!(
            &args.bare_task_requests.requests()[..2],
            &[":tasks", "--all"],
            "first task request is tasks --all"
        );
        assert_eq!(
            args.bare_task_requests.requests()[2],
            "help",
            "second task request is help"
        );
    }

    #[test]
    fn disallow_bare_unexpected_option() {
        assert!(FreightArgs::try_command_line("--all").is_err());
    }

    #[test]
    fn allow_default_tasks() {
        let args = FreightArgs::try_command_line("--trace --workers 6 --console plain").unwrap();
        assert_eq!(args.logging().log_level_filter(), LevelFilter::Trace);
        assert_eq!(args.workers(), 6);
        assert_eq!(args.logging().console, ConsoleMode::Plain);
    }

    #[test]
    fn disallow_multiple_logging() {
        assert!(FreightArgs::try_command_line("--trace --debug").is_err());
    }
}
