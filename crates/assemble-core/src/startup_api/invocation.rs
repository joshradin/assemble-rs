//! Handles standard invoking and monitoring builds

use crate::startup_api::listeners::{Listener, TaskExecutionListener};
use crate::logging::ConsoleMode;
use crate::plugins::PluginManager;
use crate::prelude::PluginAware;
use crate::version::{version, Version};
use log::LevelFilter;
use parking_lot::ReentrantMutex;
use std::collections::HashMap;
use std::env::current_dir;
use std::path::{Path, PathBuf};

/// Provides a wrapper around the assemble instance that's running this build.
#[derive(Debug)]
pub struct Assemble {
    plugins: PluginManager<Self>,
    task_listeners: Vec<Box<dyn TaskExecutionListener>>,
    version: Version,
    start_parameter: StartParameter,
}

impl Assemble {
    /// Create a new assemble instance
    pub fn new(start: StartParameter) -> Self {
        Self {
            plugins: PluginManager::new(),
            task_listeners: vec![],
            version: version(),
            start_parameter: start,
        }
    }

    /// Add a listener to the inner freight
    pub fn add_listener<T: Listener<Listened = Self>>(&mut self, listener: T) {
        listener.add_listener(self)
    }

    pub(crate) fn add_task_execution_listener<T: TaskExecutionListener + 'static>(
        &mut self,
        listener: T,
    ) {
        self.task_listeners.push(Box::new(listener))
    }

    /// Gets the current version of assemble
    pub fn assemble_version(&self) -> &Version {
        &self.version
    }

    /// Gets the start parameters used to start this build
    pub fn start_parameter(&self) -> &StartParameter {
        &self.start_parameter
    }
}

impl PluginAware for Assemble {
    fn plugin_manager(&self) -> &PluginManager<Self> {
        &self.plugins
    }

    fn plugin_manager_mut(&mut self) -> &mut PluginManager<Self> {
        &mut self.plugins
    }
}

impl Default for Assemble {
    fn default() -> Self {
        Assemble::new(StartParameter::new())
    }
}

/// A type that's aware it's part of an assemble build
pub trait AssembleAware {
    /// Get the assemble instance this value is aware of.
    fn get_assemble(&self) -> &Assemble;
}

impl AssembleAware for Assemble {
    /// Gets this [`Assemble`](Assemble) instance.
    fn get_assemble(&self) -> &Assemble {
        self
    }
}

/// The start parameters define the configuration used by an assemble instance to execute a build.
///
/// Generally corresponds to the command line options for assemble.
#[derive(Debug, Clone)]
pub struct StartParameter {
    current_dir: PathBuf,
    log_level: LevelFilter,
    mode: ConsoleMode,
    project_dir: Option<PathBuf>,
    properties: HashMap<String, String>,
    builder: String,
    task_requests: Vec<String>,
}

impl StartParameter {
    /// Creates a new instance of a start parameter with only default settings
    pub fn new() -> Self {
        Self {
            current_dir: current_dir().expect("no valid current working directory"),
            log_level: LevelFilter::Info,
            mode: ConsoleMode::Auto,
            project_dir: None,
            properties: HashMap::new(),
            builder: "".to_string(),
            task_requests: vec![],
        }
    }

    /// Gets the current directory of the start parameter, used to select the default project and
    /// find the settings file.
    pub fn current_dir(&self) -> &Path {
        &self.current_dir
    }

    /// The log level filter to use
    pub fn log_level(&self) -> LevelFilter {
        self.log_level
    }

    /// The console mode to use
    pub fn mode(&self) -> ConsoleMode {
        self.mode
    }

    /// The project directory to find the default project. If not set, defaults to the same
    /// as the current dir.
    pub fn project_dir(&self) -> PathBuf {
        self.project_dir
            .as_ref()
            .unwrap_or(&self.current_dir)
            .clone()
    }

    /// The project properties set for this build
    pub fn properties(&self) -> &HashMap<String, String> {
        &self.properties
    }

    /// A mutable reference to the project properties for this build
    pub fn properties_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.properties
    }

    /// The builder used by this project.
    pub fn builder(&self) -> &str {
        &self.builder
    }

    /// the task requests used to build this project. Contains both task names
    /// and args for said tasks
    pub fn task_requests(&self) -> &[String] {
        &self.task_requests
    }

    /// the task requests used to build this project. Contains both task names
    /// and args for said tasks
    pub fn task_requests_mut(&mut self) -> &mut Vec<String> {
        &mut self.task_requests
    }

    /// Set the current directory
    pub fn set_current_dir<P: AsRef<Path>>(&mut self, current_dir: P) {
        self.current_dir = current_dir.as_ref().to_path_buf();
    }
    /// The level filter to log
    pub fn set_log_level(&mut self, log_level: LevelFilter) {
        self.log_level = log_level;
    }

    /// Sets the console mode
    pub fn set_mode(&mut self, mode: ConsoleMode) {
        self.mode = mode;
    }

    /// Sets the project directory used to find the default project
    pub fn set_project_dir<P: AsRef<Path>>(&mut self, project_dir: P) {
        self.project_dir = Some(project_dir.as_ref().to_path_buf());
    }

    /// Sets the build type.
    pub fn set_builder(&mut self, builder: String) {
        self.builder = builder;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_assemble_version() {
        let assemble = Assemble::default();
        println!("assemble: {:#?}", assemble);
        assert_eq!(assemble.assemble_version(), &version());
    }
}
