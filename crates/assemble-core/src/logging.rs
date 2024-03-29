//! Defines different parts of the logging utilities for assemble-daemon

use crate::identifier::{ProjectId, TaskId};
use crate::unstable::text_factory::AssembleFormatter;
use atty::Stream;
use colored::Colorize;
use fern::{Dispatch, FormatCallback, Output};
use indicatif::MultiProgress;
use log::{Level, LevelFilter, Log, Record, SetLoggerError};
use merge::Merge;
use once_cell::sync::{Lazy, OnceCell};

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

use std::io::{stdout, ErrorKind, Write};
use std::path::{Path, PathBuf};

use std::ffi::OsStr;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use std::{fmt, io, thread};
use thread_local::ThreadLocal;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;

/// Provides helpful logging args for clap clis
#[derive(Debug, clap::Args, Clone, merge::Merge)]
#[clap(next_help_heading = "Log Level")]
pub struct LoggingArgs {
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

impl Default for LoggingArgs {
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

#[derive(Default, Eq, PartialEq)]
pub enum OutputType {
    #[default]
    Basic,
    TimeOnly,
    Complicated,
    Json,
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

impl LoggingArgs {
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

    /// Get the level filter from this args
    fn config_from_settings(&self) -> (LevelFilter, OutputType) {
        let level = self.log_level_filter();
        let mut output_type = if self.error {
            OutputType::Basic
        } else if self.warn {
            OutputType::Basic
        } else if self.info {
            OutputType::Basic
        } else if self.debug {
            OutputType::Basic
        } else if self.trace {
            OutputType::TimeOnly
        } else {
            OutputType::Basic
        };
        if self.json {
            output_type = OutputType::Json;
        }
        (level, output_type)
    }

    pub fn init_root_logger(&self) -> Result<Option<JoinHandle<()>>, SetLoggerError> {
        let (dispatch, handle) = self.create_logger();
        dispatch.apply().map(|_| handle)
    }

    pub fn init_root_logger_with(filter: LevelFilter, mode: OutputType) {
        Self::try_init_root_logger_with(filter, mode).expect("couldn't create dispatch");
    }

    pub fn try_init_root_logger_with(
        filter: LevelFilter,
        mode: OutputType,
    ) -> Result<(), SetLoggerError> {
        Self::create_logger_with(filter, mode, false, None).apply()
    }

    pub fn create_logger(&self) -> (Dispatch, Option<JoinHandle<()>>) {
        let (filter, output_mode) = self.config_from_settings();
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
        let (started, handle) = start_central_logger(rich);
        let central = CentralLoggerInput { sender: started };
        let output = Output::from(Box::new(central) as Box<dyn Write + Send>);
        (
            Self::create_logger_with(filter, output_mode, self.show_source, output),
            Some(handle),
        )
    }

    pub fn create_logger_with(
        filter: LevelFilter,
        mode: OutputType,
        show_source: bool,
        output: impl Into<Option<Output>>,
    ) -> Dispatch {
        let dispatch = Dispatch::new()
            .level(filter)
            .chain(output.into().unwrap_or(Output::stdout("\n")));
        match mode {
            OutputType::Json => dispatch.format(Self::json_message_format),
            other => dispatch.format(Self::message_format(other, show_source)),
        }
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
            OutputType::Basic => String::new(),
            OutputType::TimeOnly => {
                static DATE_TIME_FORMAT: &[FormatItem] =
                    format_description!("[hour]:[minute]:[second].[subsecond digits:4]");

                let time = OffsetDateTime::now_local().unwrap_or(OffsetDateTime::now_utc());
                format!(
                    "[{}] {: >7}:",
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
            _ => {
                unreachable!()
            }
        };
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

pub fn init_root_log(level: LevelFilter, mode: impl Into<Option<OutputType>>) {
    let mode = mode.into().unwrap_or_default();
    let _ = LoggingArgs::try_init_root_logger_with(level, mode);
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub enum Origin {
    Project(ProjectId),
    Task(TaskId),
    None,
}

impl From<ProjectId> for Origin {
    fn from(p: ProjectId) -> Self {
        Self::Project(p)
    }
}

impl From<TaskId> for Origin {
    fn from(t: TaskId) -> Self {
        Self::Task(t)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonMessageInfo {
    #[serde(with = "LevelDef")]
    pub level: Level,
    pub origin: Origin,
    pub message: String,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Level")]
enum LevelDef {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

static THREAD_ORIGIN: Lazy<ThreadLocal<RefCell<Origin>>> = Lazy::new(ThreadLocal::new);

fn thread_origin() -> Origin {
    THREAD_ORIGIN
        .get_or(|| RefCell::new(Origin::None))
        .borrow()
        .clone()
}

pub struct LoggingControl(());

/// Provides access to the logging control of the entire program
pub static LOGGING_CONTROL: Lazy<LoggingControl> = Lazy::new(|| LoggingControl(()));

impl LoggingControl {
    /// Sets the thread local origin
    fn use_origin(&self, new_origin: Origin) {
        trace!(
            "setting the origin for thread {:?} to {:?}",
            thread::current().id(),
            new_origin
        );
        let origin = THREAD_ORIGIN.get_or(|| RefCell::new(Origin::None));
        let mut ref_mut = origin.borrow_mut();
        *ref_mut = new_origin;
    }

    pub fn in_project(&self, project: ProjectId) {
        self.use_origin(Origin::Project(project))
        // trace!("set origin to {:?}", ref_mut);
    }

    pub fn in_task(&self, task: TaskId) {
        self.use_origin(Origin::Task(task))
        // trace!("set origin to {:?}", ref_mut);
    }

    pub fn reset(&self) {
        self.use_origin(Origin::None)
        // trace!("set origin to {:?}", ref_mut);
    }

    pub fn stop_logging(&self) {
        let lock = LOG_COMMAND_SENDER.get().unwrap();
        let sender = lock.lock().unwrap();

        sender.send(LoggingCommand::Stop).unwrap();
    }

    pub fn start_task(&self, id: &TaskId) {
        let lock = LOG_COMMAND_SENDER.get().unwrap();
        let sender = lock.lock().unwrap();

        sender
            .send(LoggingCommand::TaskStarted(id.clone()))
            .unwrap();
    }

    pub fn end_task(&self, id: &TaskId) {
        let lock = LOG_COMMAND_SENDER.get().unwrap();
        let sender = lock.lock().unwrap();

        sender.send(LoggingCommand::TaskEnded(id.clone())).unwrap();
    }

    /// Start a progress bar. Returns err if a progress bar has already been started. If Ok, the
    /// returned value is a clone of the multi-progress bar
    pub fn start_progress_bar(&self, bar: &MultiProgress) -> Result<MultiProgress, ()> {
        let lock = LOG_COMMAND_SENDER.get().unwrap();
        let sender = lock.lock().unwrap();
        sender
            .send(LoggingCommand::StartMultiProgress(bar.clone()))
            .unwrap();
        Ok(bar.clone())
    }

    /// End a progress bar if it exists
    pub fn end_progress_bar(&self) {
        let lock = LOG_COMMAND_SENDER.get().unwrap();
        let sender = lock.lock().unwrap();

        sender.send(LoggingCommand::EndMultiProgress).unwrap();
    }

    /// Run a closure within an origin context
    #[cfg(feature = "log_origin_control")]
    pub fn with_origin<O: Into<Origin>, F: FnOnce() -> R, R>(&self, origin: O, func: F) -> R {
        let origin = origin.into();

        self.use_origin(origin);
        let ret = (func)();
        self.reset();
        ret
    }

    /// Gets the origin currently set
    #[cfg(feature = "log_origin_control")]
    pub fn get_origin(&self) -> Origin {
        THREAD_ORIGIN
            .get_or(|| RefCell::new(Origin::None))
            .borrow()
            .clone()
    }
}

static CONTINUE_LOGGING: AtomicBool = AtomicBool::new(true);
static LOG_COMMAND_SENDER: OnceCell<Arc<Mutex<Sender<LoggingCommand>>>> = OnceCell::new();

fn start_central_logger(rich: bool) -> (Sender<LoggingCommand>, JoinHandle<()>) {
    let (send, recv) = channel();
    let _ = LOG_COMMAND_SENDER.set(Arc::new(Mutex::new(send.clone())));
    let handle = thread::spawn(move || {
        let mut central_logger = CentralLoggerOutput::new();
        loop {
            let command = match recv.recv() {
                Ok(s) => s,
                Err(_) => break,
            };

            match command {
                LoggingCommand::LogString(o, s) => {
                    central_logger.add_output(o, &s);
                    central_logger.flush_current_origin();
                }
                LoggingCommand::Flush => central_logger.flush(),
                LoggingCommand::Stop => {
                    break;
                }
                LoggingCommand::TaskStarted(s) => {
                    if !rich {
                        central_logger.add_output(Origin::Task(s), "");
                        central_logger.flush_current_origin();
                    }
                }
                LoggingCommand::TaskEnded(_s) => {}
                LoggingCommand::TaskStatus(_, _) => {}
                LoggingCommand::StartMultiProgress(b) => {
                    central_logger.start_progress_bar(&b).unwrap();
                }
                LoggingCommand::EndMultiProgress => {
                    central_logger.end_progress_bar();
                }
            }
        }

        central_logger.flush();
    });
    LOGGING_CONTROL.reset();
    (send, handle)
}

pub enum LoggingCommand {
    LogString(Origin, String),
    TaskStarted(TaskId),
    TaskEnded(TaskId),
    TaskStatus(TaskId, String),
    StartMultiProgress(MultiProgress),
    EndMultiProgress,
    Flush,
    Stop,
}

pub struct CentralLoggerInput {
    sender: Sender<LoggingCommand>,
}

assert_impl_all!(CentralLoggerInput: Send, Write);

impl io::Write for CentralLoggerInput {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let string = String::from_utf8_lossy(buf).to_string();

        let origin = thread_origin();
        // println!("sending from origin: {origin:?}");
        self.sender
            .send(LoggingCommand::LogString(origin, string))
            .map_err(|e| io::Error::new(ErrorKind::Interrupted, e))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.sender
            .send(LoggingCommand::Flush)
            .map_err(|e| io::Error::new(ErrorKind::Interrupted, e))
    }
}

#[derive(Debug)]
pub struct CentralLoggerOutput {
    saved_output: HashMap<Origin, String>,
    origin_buffers: HashMap<Origin, String>,
    origin_queue: VecDeque<Origin>,
    previous: Option<Origin>,
    last_query: Option<Instant>,
    progress_bar: Option<MultiProgress>,
}

impl CentralLoggerOutput {
    pub fn new() -> Self {
        Self {
            saved_output: Default::default(),
            origin_buffers: Default::default(),
            origin_queue: Default::default(),
            previous: None,
            last_query: None,
            progress_bar: None,
        }
    }

    pub fn add_output(&mut self, origin: Origin, msg: &str) {
        let buffer = self.origin_buffers.entry(origin.clone()).or_default();
        *buffer = format!("{}{}", buffer, msg);
        if let Some(front) = self.origin_queue.front() {
            if front != &origin {
                if self.last_query.unwrap().elapsed() >= Duration::from_millis(100) {
                    self.origin_queue.pop_front();
                }
                self.origin_queue.push_back(origin);
            }
            self.last_query = Some(Instant::now());
        } else {
            self.origin_queue.push_back(origin);
        }
    }

    /// Flushes current lines from an origin
    pub fn flush_current_origin(&mut self) {
        self.last_query = Some(Instant::now());
        let origin = self.origin_queue.front().cloned().unwrap_or(Origin::None);

        if Some(&origin) != self.previous.as_ref() {
            match &origin {
                Origin::Project(p) => {
                    self.println(format!(
                        "{}",
                        AssembleFormatter::default()
                            .project_status(p, "configuring")
                            .unwrap()
                    ))
                    .unwrap();
                }
                Origin::Task(t) => {
                    self.println(format!(
                        "{}",
                        AssembleFormatter::default().task_status(t, "").unwrap()
                    ))
                    .unwrap();
                }
                Origin::None => {}
            }
        }

        self.previous = Some(origin.clone());
        let printer = self.logger_stdout();
        let saved = self.saved_output.entry(origin.clone()).or_default();
        if let Some(buffer) = self.origin_buffers.get_mut(&origin) {
            let mut lines = Vec::new();
            while let Some(position) = buffer.chars().position(|c| c == '\n') {
                let head = &buffer[..position];
                let tail = buffer.get((position + 1)..).unwrap_or_default();

                lines.push(head.to_string());

                *buffer = tail.to_string();
            }

            for line in lines {
                if !(saved.trim().is_empty() && line.trim().is_empty()) {
                    printer.println(&line).unwrap();
                    *saved = format!("{}{}", saved, line);
                }
            }

            if buffer.trim().is_empty() {
                self.origin_queue.pop_front();
            }
        }
    }

    pub fn flush(&mut self) {
        let printer = self.logger_stdout();
        let drained = self.origin_queue.drain(..).collect::<Vec<_>>();
        for origin in drained {
            if let Some(str) = self.origin_buffers.get_mut(&origin) {
                printer.println(format!("{origin:?}: {}", str)).unwrap();
                str.clear();
            }
        }
        stdout().flush().unwrap();
    }

    pub fn println(&self, string: impl AsRef<str>) -> io::Result<()> {
        match &self.progress_bar {
            None => {
                writeln!(stdout(), "{}", string.as_ref())
            }
            Some(p) => p.println(string),
        }
    }

    pub fn logger_stdout(&self) -> LoggerStdout {
        LoggerStdout {
            progress: self.progress_bar.clone(),
        }
    }

    /// Start a progress bar. Returns err if a progress bar has already been started. If Ok, the
    /// returned value is a clone of the multi-progress bar
    pub fn start_progress_bar(&mut self, bar: &MultiProgress) -> Result<MultiProgress, ()> {
        self.progress_bar = Some(bar.clone());
        Ok(bar.clone())
    }

    /// End a progress bar if it exists
    pub fn end_progress_bar(&mut self) {
        let replaced = std::mem::replace(&mut self.progress_bar, None);
        if let Some(replaced) = replaced {
            replaced.clear().unwrap();
        }
    }
}

pub struct LoggerStdout {
    progress: Option<MultiProgress>,
}

impl LoggerStdout {
    pub fn println(&self, string: impl AsRef<str>) -> io::Result<()> {
        match &self.progress {
            None => {
                writeln!(stdout(), "{}", string.as_ref())
            }
            Some(p) => p.println(string),
        }
    }
}
