//! Defines different parts of the logging utilities for assemble-daemon

use crate::identifier::{ProjectId, TaskId};
use fern::{Dispatch, FormatCallback, Output};
use indicatif::ProgressBar;
use log::{log, logger, set_logger, Level, LevelFilter, Log, Metadata, Record, SetLoggerError};
use once_cell::sync::{Lazy, OnceCell};
use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::fmt::{Display, format, Formatter};
use std::io::{BufRead, BufReader, ErrorKind, stdout, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{JoinHandle, ThreadId};
use std::{fmt, io, thread};
use std::cell::{Cell, RefCell};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};
use atty::Stream;
use thread_local::ThreadLocal;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::{format_description, OffsetDateTime};
use crate::text_factory::AssembleFormatter;

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

    /// Outputs everything as json
    #[clap(long)]
    pub json: bool,

    #[clap(long, value_enum, default_value_t = ConsoleMode::Auto)]
    console: ConsoleMode
}

#[derive(Default, Eq, PartialEq)]
pub enum OutputType {
    #[default]
    Basic,
    TimeOnly,
    Complicated,
    Json
}

#[derive(Debug, Copy, Clone, clap::ValueEnum)]
#[repr(u8)]
pub enum ConsoleMode {
    Auto,
    Advanced,
    Plain
}



impl ConsoleMode {

    pub fn resolve(self) -> Self {
        match &self {
            ConsoleMode::Auto => {
                if atty::is(Stream::Stdout) {
                    ConsoleMode::Advanced
                } else {
                    ConsoleMode::Plain
                }
            }
            ConsoleMode::Advanced => { self }
            ConsoleMode::Plain => { self }
        }
    }
}

impl LoggingArgs {
    /// Get the level filter from this args
    fn config_from_settings(&self) -> (LevelFilter, OutputType) {
        let mut out = if self.error {
            (LevelFilter::Error, OutputType::Basic)
        } else if self.warn {
            (LevelFilter::Warn, OutputType::Basic)
        } else if self.info {
            (LevelFilter::Info, OutputType::Basic)
        } else if self.debug {
            (LevelFilter::Debug, OutputType::Basic)
        } else if self.trace {
            (LevelFilter::Trace, OutputType::TimeOnly)
        } else {
            (LevelFilter::Info, OutputType::Basic)
        };
        if self.json {
            out.1 = OutputType::Json;
        }
        out
    }

    pub fn init_root_logger(&self) -> Result<Option<JoinHandle<()>>, SetLoggerError> {
        let (dispatch, handle) = self.create_logger();
        dispatch.apply()
            .map(|_| handle)
    }

    pub fn init_root_logger_with(filter: LevelFilter, mode: OutputType) {
        Self::try_init_root_logger_with(filter, mode)
            .expect("couldn't create dispatch");
    }

    pub fn try_init_root_logger_with(
        filter: LevelFilter,
        mode: OutputType,
    ) -> Result<(), SetLoggerError> {
        Self::create_logger_with(filter, mode, None)
            .apply()
    }

    pub fn create_logger(&self) -> (Dispatch, Option<JoinHandle<()>>) {
        let (filter, output_mode) = self.config_from_settings();
        let mut handle = None;
        let output = match self.console.resolve() {
            ConsoleMode::Auto => { unreachable!()}
            ConsoleMode::Advanced => {
                let (started, made_handle) = start_central_logger();
                handle = Some(made_handle);
                let central = CentralLoggerInput {
                    sender: started
                };
                Some(Output::from(Box::new(central) as Box<dyn Write + Send>))
            }
            ConsoleMode::Plain => { None }
        };
        (Self::create_logger_with(filter, output_mode, output), handle)
    }

    pub fn create_logger_with(filter: LevelFilter, mode: OutputType, output: impl Into<Option<Output>>) -> Dispatch {
        let mut dispatch = Dispatch::new()
            .level(filter)
            .chain(output.into().unwrap_or(Output::stdout("\n")));
        match mode {
            OutputType::Json => {
                dispatch.format(Self::json_message_format)
            }
            other => {
                dispatch.format(Self::message_format(other, false))
            }
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
                    message
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
            message
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
            _ => {unreachable!()}
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

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub enum Origin {
    Project(ProjectId),
    Task(TaskId),
    None
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonMessageInfo {
    #[serde(with = "LevelDef")]
    pub level: Level,
    pub origin: Origin,
    pub message: String
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

static THREAD_ORIGIN: Lazy<ThreadLocal<RefCell<Origin>>> = Lazy::new(|| ThreadLocal::new() );

fn thread_origin() -> Origin {
    THREAD_ORIGIN.get_or(|| RefCell::new(Origin::None))
        .borrow()
        .clone()
}

pub fn in_project(project: ProjectId) {
    let origin = THREAD_ORIGIN.get_or(|| RefCell::new(Origin::None));
    let mut ref_mut = origin.borrow_mut();
    *ref_mut = Origin::Project(project);
    // trace!("set origin to {:?}", ref_mut);
}

pub fn in_task(task: TaskId) {
    let origin = THREAD_ORIGIN.get_or(|| RefCell::new(Origin::None));
    let mut ref_mut = origin.borrow_mut();
    *ref_mut = Origin::Task(task);
    // trace!("set origin to {:?}", ref_mut);
}

pub fn reset() {
    let origin = THREAD_ORIGIN.get_or(|| RefCell::new(Origin::None));
    let mut ref_mut = origin.borrow_mut();
    *ref_mut = Origin::None;
    // trace!("set origin to {:?}", ref_mut);
}


pub fn stop_logging() {
    let lock = LOG_COMMAND_SENDER.get()
        .unwrap();
    let sender = lock
        .lock()
        .unwrap();

    sender.send(LoggingCommand::Stop);
}

static CONTINUE_LOGGING: AtomicBool = AtomicBool::new(true);
static LOG_COMMAND_SENDER: OnceCell<Arc<Mutex<Sender<LoggingCommand>>>> = OnceCell::new();

fn start_central_logger() -> (Sender<LoggingCommand>, JoinHandle<()>) {
    let (send, recv) = channel();
    LOG_COMMAND_SENDER.set(Arc::new(Mutex::new(send.clone())));
    let handle = thread::spawn(move || {
        let mut central_logger = CentralLoggerOutput::new();
        loop {
            let command = match recv.try_recv() {
                Ok(s) => { s }
                Err(TryRecvError::Empty) => {
                    continue
                }
                Err(TryRecvError::Disconnected) => break,
            };

            match command {
                LoggingCommand::LogString(o, s) => {
                    central_logger.add_output(o, &s);
                    central_logger.flush_current_origin();
                }
                LoggingCommand::Flush => {
                    central_logger.flush()
                }
                LoggingCommand::Stop => {
                    break;
                }
            }
        }

        central_logger.flush();
    });
    reset();
    (send, handle)
}

pub enum LoggingCommand {
    LogString(Origin, String),
    Flush,
    Stop,
}

pub struct CentralLoggerInput {
    sender: Sender<LoggingCommand>
}

assert_impl_all!(CentralLoggerInput: Send, Write);

impl io::Write for CentralLoggerInput {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let string = String::from_utf8_lossy(buf).to_string();

        let origin = thread_origin();
        // println!("sending from origin: {origin:?}");
        self.sender.send(LoggingCommand::LogString(origin, string)).map_err(|e| io::Error::new(ErrorKind::Interrupted, e))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.sender.send(LoggingCommand::Flush).map_err(|e| io::Error::new(ErrorKind::Interrupted, e))
    }
}

#[derive(Debug)]
pub struct CentralLoggerOutput {
    origin_buffers: HashMap<Origin, String>,
    origin_queue: VecDeque<Origin>,
    previous: Option<Origin>,
    last_query: Option<Instant>,
}

impl CentralLoggerOutput {
    pub fn new() -> Self {
        Self { origin_buffers: Default::default(), origin_queue: Default::default(), previous: None, last_query: None }
    }

    pub fn add_output(&mut self, origin: Origin, msg: &str) {
        let buffer = self.origin_buffers.entry(origin.clone())
            .or_default();
        *buffer = format!("{}{}", buffer, msg);
        if let Some(front) = self.origin_queue.front() {
            if front != &origin {
                if self.last_query.unwrap().elapsed() >= Duration::from_millis(1000){
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
            println!();
            match &origin {
                Origin::Project(p) => {
                    println!("{}", AssembleFormatter::default().project_status(p, "configuring").unwrap());
                }
                Origin::Task(t) => {
                    println!("{}", AssembleFormatter::default().task_status(t, "").unwrap());
                }
                Origin::None => {}
            }
        }

        self.previous = Some(origin.clone());

        if let Some(buffer) = self.origin_buffers.get_mut(&origin) {
            let mut lines = Vec::new();
            while let Some(position) = buffer.chars().position(|c| c == '\n') {
                let head = &buffer[..position];
                let tail = buffer.get((position+1)..).unwrap_or_default();

                lines.push(head.to_string());

                *buffer = tail.to_string();
            }

            for line in lines {
                println!("{}", line);
            }

            if buffer.trim().is_empty() {
                self.origin_queue.pop_front();

            }
        }
    }

    pub fn flush(&mut self) {
        let drained = self.origin_queue.drain(..).collect::<Vec<_>>();
        for origin in drained {
            if let Some(str) = self.origin_buffers.get_mut(&origin) {
                println!("{origin:?}: {}", str);
                str.clear();
            }
        }
        stdout().flush().unwrap();
    }
}


