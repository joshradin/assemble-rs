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

use opts::LoggingOpts;
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

pub mod opts;

pub fn init_root_log(level: LevelFilter, mode: impl Into<Option<OutputType>>) {
    let mode = mode.into().unwrap_or_default();
    let _ = LoggingOpts::try_init_root_logger_with(level, mode);
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
