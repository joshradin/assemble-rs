use std::collections::HashMap;
use std::str::FromStr;
use assemble_core::logging::LoggingArgs;
use clap::Parser;
use std::num::NonZeroUsize;
use assemble_core::identifier::TaskId;
use crate::ProjectProperties;

#[derive(Debug)]
pub enum TaskArg {
    Task(String),
    Flag(String, Option<String>)
}

impl FromStr for TaskArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('-') {
            let no_hyphen = s.trim_start_matches('-');
            if no_hyphen.contains('=') {
                let (flag, value) = no_hyphen.split_once('=').unwrap();
                Ok(Self::Flag(flag.to_string(), Some(value.to_string())))
            } else {
                Ok(Self::Flag(no_hyphen.to_string(), None))
            }
        } else {
            Ok(Self::Task(s.to_string()))
        }
    }
}

#[derive(Debug)]
pub struct TaskRequest {
    pub task: String,
    pub flags: HashMap<String, Option<String>>
}

pub struct TaskRequests {
    task_to_request: HashMap<String, TaskRequest>
}

impl TaskRequests {
    pub fn tasks(&self) -> impl Iterator<Item=String> + '_ {
        self.task_to_request.keys().cloned()
    }

    pub fn flags_for_tasks(&self, task: &TaskId) -> Option<HashMap<String, Option<String>>> {
        for key in self.task_to_request.keys() {
            if task.is_shorthand(key) {
                return self.task_to_request.get(key).map(|t| t.flags.clone());
            }
        }
        None
    }
}


/// The args to run Freight
#[derive(Debug, Parser)]
#[clap(about)]
#[clap(allow_hyphen_values = true)]
pub struct FreightArgs {
    /// Tasks to be run
    pub tasks: Vec<TaskArg>,
    /// Project properties. Set using -P or --prop
    #[clap(flatten)]
    pub properties: ProjectProperties,
    /// Log level to run freight in.
    #[clap(flatten)]
    pub log_level: LoggingArgs,
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

    pub fn tasks(&self) -> Vec<&String> {
        self.tasks.iter()
            .filter_map(|r|
                if let TaskArg::Task(t) = r {
                    Some(t)
                } else {
                    None
                }
            )
            .collect()
    }


}

impl<S: AsRef<str>> FromIterator<S> for FreightArgs {
    fn from_iter<T: IntoIterator<Item = S>>(iter: T) -> Self {
        let mut args = vec![String::new()];
        args.extend(iter.into_iter().map(|s: S| s.as_ref().to_string()));

        FreightArgs::parse_from(args)
    }
}
