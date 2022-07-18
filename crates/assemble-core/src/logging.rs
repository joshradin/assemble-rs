//! Defines different parts of the logging utilities for assemble-daemon

use crate::identifier::TaskId;
use fern::{Dispatch, FormatCallback};
use indicatif::ProgressBar;
use log::{log, logger, set_logger, Level, LevelFilter, Log, Metadata, Record, SetLoggerError};
use once_cell::sync::{Lazy, OnceCell};
use std::any::Any;
use std::collections::HashMap;
use std::fmt::format;
use std::io::stdout;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::thread::ThreadId;
use std::{fmt, thread};
use time::format_description::FormatItem;
use time::macros::format_description;
use time::{format_description, OffsetDateTime};

/// Provides helpful logging args for clap clis
#[derive(Debug, clap::Args)]
#[clap(next_help_heading = "LOGGING")]
pub struct LoggingArgs {
    /// Show the source of a logging statement when running in any non complicated mode
    #[clap(long)]
    #[clap(conflicts_with_all(&["trace"]))]
    show_source: bool,

    /// Only display error level log messages
    #[clap(short, long)]
    #[clap(conflicts_with_all(&["warn", "info", "debug", "trace"]))]
    #[clap(display_order = 1)]
    error: bool,

    /// Display warning and above level log messages
    #[clap(short, long)]
    #[clap(conflicts_with_all(&["error", "info", "debug", "trace"]))]
    #[clap(display_order = 2)]
    warn: bool,

    /// Display info and above level log messages
    #[clap(short, long)]
    #[clap(conflicts_with_all(&["error", "warn", "debug", "trace"]))]
    #[clap(display_order = 3)]
    info: bool,

    /// Display debug and above level log messages
    #[clap(long)]
    #[clap(conflicts_with_all(&["error", "warn", "info", "trace"]))]
    #[clap(display_order = 4)]
    debug: bool,

    /// Display trace and above level log messages
    #[clap(long)]
    #[clap(conflicts_with_all(&["error", "warn", "info", "debug"]))]
    #[clap(display_order = 5)]
    trace: bool,
}

#[derive(Default)]
pub enum OutputType {
    #[default]
    Basic,
    TimeOnly,
    Complicated,
}

impl LoggingArgs {
    /// Get the level filter from this args
    fn config_from_settings(&self) -> (LevelFilter, OutputType) {
        if self.error {
            (LevelFilter::Error, OutputType::Basic)
        } else if self.warn {
            (LevelFilter::Warn, OutputType::Basic)
        } else if self.info {
            (LevelFilter::Info, OutputType::TimeOnly)
        } else if self.debug {
            (LevelFilter::Debug, OutputType::TimeOnly)
        } else if self.trace {
            (LevelFilter::Trace, OutputType::Complicated)
        } else {
            (LevelFilter::Info, OutputType::Basic)
        }
    }

    pub fn init_root_logger(&self) -> bool {
        self.create_logger().apply().is_ok()
    }

    pub fn init_root_logger_with(filter: LevelFilter, mode: OutputType) {
        Dispatch::new()
            .format(Self::message_format(mode, false))
            .level(filter)
            .chain(stdout())
            .apply()
            .expect("couldn't create dispatch");
    }

    pub fn try_init_root_logger_with(
        filter: LevelFilter,
        mode: OutputType,
    ) -> Result<(), SetLoggerError> {
        Dispatch::new()
            .format(Self::message_format(mode, false))
            .level(filter)
            .chain(stdout())
            .apply()
    }

    pub fn create_logger(&self) -> Dispatch {
        let (filter, output_mode) = self.config_from_settings();
        Dispatch::new()
            .format(Self::message_format(output_mode, self.show_source))
            .level(filter)
            .chain(stdout())
    }

    fn message_format(
        output_mode: OutputType,
        show_source: bool,
    ) -> impl Fn(FormatCallback, &fmt::Arguments, &log::Record) + Sync + Send + 'static {
        move |out, message, record| {
            out.finish(format_args!(
                "{}{}",
                {
                    let prefix = Self::format_prefix(&output_mode, record, show_source);
                    if prefix.is_empty() {
                        prefix
                    } else {
                        format!("{} ", prefix)
                    }
                },
                message
            ))
        }
    }

    fn format_prefix(output_mode: &OutputType, record: &Record, show_source: bool) -> String {
        use colored::Colorize;
        let mut level_string = record.level().to_string().to_lowercase();

        level_string = match record.level() {
            Level::Error => level_string.red().to_string(),
            Level::Warn => level_string.yellow().to_string(),
            Level::Info => level_string.green().to_string(),
            Level::Debug => level_string.blue().to_string(),
            Level::Trace => level_string.bright_black().to_string(),
        };
        let output = match output_mode {
            OutputType::Basic => {
                if record.level() < Level::Info {
                    format!("{:<7}", format!("{}:", level_string.to_lowercase()))
                } else {
                    format!("")
                }
            }
            OutputType::TimeOnly => {
                static DATE_TIME_FORMAT: &[FormatItem] =
                    format_description!("[hour]:[minute]:[second].[subsecond digits:4]");

                let time = OffsetDateTime::now_local().unwrap_or(OffsetDateTime::now_utc());
                format!(
                    "[{}] {:>6}:",
                    time.format(DATE_TIME_FORMAT).unwrap(),
                    level_string
                )
            }
            OutputType::Complicated => {
                static DATE_TIME_FORMAT: &[FormatItem] = format_description!("[year]/[month]/[day] [hour]:[minute]:[second].[subsecond digits:4] [offset_hour sign:mandatory padding:none] UTC");

                let time = OffsetDateTime::now_utc();
                let file_path = Path::new(record.file().unwrap_or("unknown"));
                format!(
                    "[{} {}{} {}]",
                    time.format(DATE_TIME_FORMAT).unwrap(),
                    file_path.file_name().and_then(|s| s.to_str()).unwrap(),
                    record
                        .line()
                        .map(|l| format!(":{l}"))
                        .unwrap_or("".to_string()),
                    level_string
                )
            }
        };
        if show_source {
            if let Some(source) = record.module_path() {
                let line = record.line().map(|i| format!(":{}", i)).unwrap_or_default();
                let source = format!("({source}{line})").italic();
                format!("{source} {output}")
            } else {
                output
            }
        } else {
            output
        }
    }
}

pub fn init_root_log(level: LevelFilter, mode: impl Into<Option<OutputType>>) {
    let mode = mode.into().unwrap_or_default();
    let _ = LoggingArgs::try_init_root_logger_with(level, mode);
}

/// Modifies the logging output of the program by intercepting stdout.
#[derive(Debug)]
pub struct TaskProgressDisplay {
    inner: Arc<RwLock<TaskProgressDisplayInner>>,
}

#[derive(Debug)]
struct TaskProgressDisplayInner {
    total_tasks: usize,
    completed_tasks: usize,
    running_tasks: Vec<TaskId>,
}

pub struct TaskProgress {
    progress: ProgressBar,
}

pub struct ThreadBasedLogger {
    thread_id_to_task: RwLock<HashMap<ThreadId, TaskId>>,
}

impl ThreadBasedLogger {
    pub fn new() -> Self {
        Self {
            thread_id_to_task: Default::default(),
        }
    }

    pub fn logger() -> &'static Self {
        ROOT_LOGGER.get().unwrap()
    }

    pub fn register_thread_to_task(&self, id: &TaskId) {
        let mut result = self.thread_id_to_task.write().unwrap();
        let thread_id = thread::current().id();
        result.insert(thread_id, id.clone());
    }

    pub fn deregister_thread_to_task(&self) {
        let mut result = self.thread_id_to_task.write().unwrap();
        let thread_id = thread::current().id();
        result.remove(&thread_id);
    }

    pub fn apply(self) -> Result<(), Error> {
        ROOT_LOGGER.set(self).map_err(|_| Error::RootAlreadySet)?;
        Ok(())
    }
}

static ROOT_LOGGER: OnceCell<ThreadBasedLogger> = OnceCell::new();

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Root logger already set")]
    RootAlreadySet,
    #[error(transparent)]
    SetLoggerError(#[from] SetLoggerError),
}
