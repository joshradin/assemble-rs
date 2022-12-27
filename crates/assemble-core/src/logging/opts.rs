use crate::logging;
use crate::logging::{CentralLoggerInput, JsonMessageInfo, Origin};
use atty::Stream;
use colored::Colorize;
use fern::{Dispatch, FormatCallback, Output};
use log::{Level, LevelFilter, Record, SetLoggerError};
use merge::Merge;
use std::fmt;
use std::io::Write;
use std::thread::JoinHandle;
use time::macros::format_description;

/// Provides helpful logging args for clap clis
#[derive(Debug, clap::Args, Clone, merge::Merge)]
#[clap(next_help_heading = "Log Level")]
pub struct LoggingOpts {
    /// Only display error level log messages
    #[clap(short, long)]
    #[clap(conflicts_with_all(&["warn", "info", "debug", "trace"]))]
    #[clap(display_order = 1)]
    #[clap(global = true)]
    #[merge(strategy = merge::bool::overwrite_false)]
    error: bool,

    /// Display warning and above level log messages
    #[clap(short, long)]
    #[clap(conflicts_with_all(&["error", "info", "debug", "trace"]))]
    #[clap(display_order = 2)]
    #[clap(global = true)]
    #[merge(strategy = merge::bool::overwrite_false)]
    warn: bool,

    /// Display info and above level log messages
    #[clap(short, long)]
    #[clap(conflicts_with_all(&["error", "warn", "debug", "trace"]))]
    #[clap(display_order = 3)]
    #[clap(global = true)]
    #[merge(strategy = merge::bool::overwrite_false)]
    info: bool,

    /// Display debug and above level log messages
    #[clap(long, short)]
    #[clap(conflicts_with_all(&["error", "warn", "info", "trace"]))]
    #[clap(display_order = 4)]
    #[clap(global = true)]
    #[merge(strategy = merge::bool::overwrite_false)]
    debug: bool,

    /// Display trace and above level log messages
    #[clap(long)]
    #[clap(conflicts_with_all(&["error", "warn", "info", "debug"]))]
    #[clap(display_order = 5)]
    #[clap(global = true)]
    #[merge(strategy = merge::bool::overwrite_false)]
    trace: bool,

    /// Show the source of a logging statement when running in any non complicated mode
    #[clap(long)]
    #[clap(help_heading = "Logging Settings")]
    #[clap(global = true)]
    #[merge(strategy =merge::bool::overwrite_false)]
    pub show_source: bool,

    /// Outputs everything as json
    #[clap(long)]
    #[clap(help_heading = "Logging Settings")]
    #[clap(global = true)]
    #[merge(strategy = merge::bool::overwrite_false)]
    pub json: bool,

    /// The console output mode.
    #[clap(long, value_enum, default_value_t = ConsoleMode::Auto)]
    #[clap(help_heading = "Logging Settings")]
    #[clap(global = true)]
    pub console: ConsoleMode,
}

impl Default for LoggingOpts {
    fn default() -> Self {
        Self {
            show_source: false,
            error: false,
            warn: false,
            info: false,
            debug: true,
            trace: false,
            json: false,
            console: ConsoleMode::Plain,
        }
    }
}

#[derive(Debug, Copy, Clone, clap::ValueEnum, Eq, PartialEq)]
#[repr(u8)]
pub enum ConsoleMode {
    Auto,
    Rich,
    Plain,
}

impl Merge for ConsoleMode {
    fn merge(&mut self, other: Self) {
        if self == &Self::Auto {
            *self = other;
        }
    }
}

impl ConsoleMode {
    pub fn resolve(self) -> Self {
        match &self {
            ConsoleMode::Auto => {
                if atty::is(Stream::Stdout) {
                    ConsoleMode::Rich
                } else {
                    ConsoleMode::Plain
                }
            }
            ConsoleMode::Rich => self,
            ConsoleMode::Plain => self,
        }
    }
}

impl LoggingOpts {
    /// Gets the log level
    pub fn log_level_filter(&self) -> LevelFilter {
        if self.error {
            LevelFilter::Error
        } else if self.warn {
            LevelFilter::Warn
        } else if self.info {
            LevelFilter::Info
        } else if self.debug {
            LevelFilter::Debug
        } else if self.trace {
            LevelFilter::Trace
        } else {
            LevelFilter::Info
        }
    }

    pub fn init_root_logger(&self) -> Result<Option<JoinHandle<()>>, SetLoggerError> {
        let (dispatch, handle) = self.create_logger();
        dispatch.apply().map(|_| handle)
    }

    pub fn init_root_logger_with(filter: LevelFilter) {
        Self::try_init_root_logger_with(filter).expect("couldn't create dispatch");
    }

    pub fn try_init_root_logger_with(filter: LevelFilter) -> Result<(), SetLoggerError> {
        Self::create_logger_with(filter, false, None).apply()
    }

    pub fn create_logger(&self) -> (Dispatch, Option<JoinHandle<()>>) {
        let filter = self.log_level_filter();
        let rich: bool = match self.console.resolve() {
            ConsoleMode::Auto => {
                unreachable!()
            }
            ConsoleMode::Rich => true,
            ConsoleMode::Plain => false,
        };
        if !rich {
            colored::control::set_override(false);
        }
        let (started, handle) = logging::start_central_logger(rich);
        let central = CentralLoggerInput { sender: started };
        let output = Output::from(Box::new(central) as Box<dyn Write + Send>);
        (
            Self::create_logger_with(filter, self.show_source, output),
            Some(handle),
        )
    }

    pub fn create_logger_with(
        filter: LevelFilter,
        show_source: bool,
        output: impl Into<Option<Output>>,
    ) -> Dispatch {
        let dispatch = Dispatch::new()
            .level(filter)
            .chain(output.into().unwrap_or(Output::stdout("\n")));
        dispatch.format(Self::message_format(show_source))
    }

    fn message_format(
        show_source: bool,
    ) -> impl Fn(FormatCallback, &fmt::Arguments, &log::Record) + Sync + Send + 'static {
        move |out, message, record| {
            out.finish(format_args!(
                "{}{}",
                {
                    let prefix = Self::format_prefix(&record, show_source);
                    if prefix.is_empty() {
                        prefix
                    } else {
                        format!("{} ", prefix)
                    }
                },
                match record.level() {
                    Level::Error => {
                        format!("{}", message.to_string().red())
                    }
                    Level::Warn => {
                        format!("{}", message.to_string().yellow())
                    }
                    Level::Info | Level::Debug => {
                        message.to_string()
                    }
                    Level::Trace => {
                        format!("{}", message.to_string().bright_blue())
                    }
                }
            ))
        }
    }

    fn json_message_format(format: FormatCallback, args: &fmt::Arguments, record: &log::Record) {
        let message = format!("{}", args);
        let level = record.level();
        let origin = Origin::None;

        let message_info = JsonMessageInfo {
            level,
            origin,
            message,
        };

        let as_string = serde_json::to_string(&message_info).unwrap();

        format.finish(format_args!("{}", as_string));
    }

    fn format_prefix(record: &Record, show_source: bool) -> String {
        use colored::Colorize;
        use std::ffi::OsStr;
        use std::path::{Path, PathBuf};
        use time::OffsetDateTime;
        let mut level_string = record.level().to_string().to_lowercase();

        level_string = match record.level() {
            Level::Error => level_string.red().to_string(),
            Level::Warn => level_string.yellow().to_string(),
            Level::Info => level_string.green().to_string(),
            Level::Debug => level_string.blue().to_string(),
            Level::Trace => level_string.bright_black().to_string(),
        };
        let output = "".to_string();
        if show_source {
            if let Some((module, file)) = record.module_path().zip(record.file()) {
                let line = record.line().map(|i| format!(":{}", i)).unwrap_or_default();
                let crate_name = module.split("::").next().unwrap();
                let source: PathBuf = Path::new(file)
                    .iter()
                    .skip_while(|&p| p != OsStr::new("src"))
                    .skip(1)
                    .collect();

                let source = format!(
                    "({crate_name} :: {source}{line})",
                    source = source.to_string_lossy()
                )
                .italic();

                format!("{source} {output}")
            } else {
                format!("(<unknown source>) {output}")
            }
        } else {
            output
        }
    }
}
