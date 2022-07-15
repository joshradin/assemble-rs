//! Defines different parts of the logging utilities for assemble-daemon

use crate::identifier::TaskId;
use fern::{Dispatch, FormatCallback};
use indicatif::ProgressBar;
use log::{log, set_logger, Level, LevelFilter, Log, Metadata, Record, SetLoggerError};
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

pub enum OutputType {
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
            (LevelFilter::Debug, OutputType::Complicated)
        } else if self.trace {
            (LevelFilter::Trace, OutputType::Complicated)
        } else {
            (LevelFilter::Info, OutputType::Basic)
        }
    }

    pub fn init_root_logger(&self) {
        let (filter, output_mode) = self.config_from_settings();

        Dispatch::new()
            .format(self.message_format(output_mode))
            .level(filter)
            .chain(stdout())
            .apply();
    }

    pub fn create_logger(&self) -> Dispatch {
        let (filter, output_mode) = self.config_from_settings();
        Dispatch::new()
            .format(self.message_format(output_mode))
            .level(filter)
            .chain(stdout())
    }

    fn message_format(
        &self,
        output_mode: OutputType,
    ) -> impl Fn(FormatCallback, &fmt::Arguments, &log::Record) + Sync + Send + 'static {
        move |out, message, record| {
            out.finish(format_args!(
                "{} {}",
                Self::format_prefix(&output_mode, record),
                message
            ))
        }
    }

    fn format_prefix(output_mode: &OutputType, record: &Record) -> String {
        use colored::Colorize;
        let mut level_string = record.level().to_string().to_lowercase();
        static DATE_TIME_FORMAT: &[FormatItem] = format_description!("[year]/[month]/[day] [hour]:[minute]:[second].[subsecond digits:4] [offset_hour sign:mandatory padding:none] UTC");

        level_string = match record.level() {
            Level::Error => level_string.red().to_string(),
            Level::Warn => level_string.yellow().to_string(),
            Level::Info => level_string.green().to_string(),
            Level::Debug => level_string.blue().to_string(),
            Level::Trace => level_string.bright_black().to_string(),
        };
        match output_mode {
            OutputType::Basic => {
                format!("{}:", level_string.to_lowercase())
            }
            OutputType::TimeOnly => {
                let time = OffsetDateTime::now_local().unwrap_or(OffsetDateTime::now_utc());
                format!(
                    "[{}] {}:",
                    time.format(DATE_TIME_FORMAT).unwrap(),
                    level_string
                )
            }
            OutputType::Complicated => {
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
        }
    }
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
