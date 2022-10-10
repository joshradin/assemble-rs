//! Handles standard invoking and monitoring builds

use crate::logging::{ConsoleMode, LoggingArgs};
use crate::plugins::PluginManager;
use crate::prelude::listeners::TaskExecutionGraphListener;
use crate::prelude::PluginAware;
use crate::private::Sealed;
use crate::project::requests::TaskRequests;
use crate::project::ProjectResult;
use crate::startup_api::execution_graph::ExecutionGraph;
use crate::startup_api::listeners::{Listener, TaskExecutionListener};
use crate::version::{version, Version};
use log::LevelFilter;
use once_cell::sync::OnceCell;
use parking_lot::{ReentrantMutex, RwLock};
use std::collections::HashMap;
use std::env::current_dir;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Provides a wrapper around the assemble instance that's running this build.
#[derive(Debug)]
pub struct Assemble {
    plugins: PluginManager<Assemble>,
    task_listeners: Vec<Box<dyn TaskExecutionListener>>,
    task_graph_listeners: Vec<Box<dyn TaskExecutionGraphListener>>,
    version: Version,
    start_parameter: StartParameter,
    graph: RwLock<OnceCell<ExecutionGraph>>,
}

impl Assemble {
    /// Create a new assemble instance
    pub fn new(start: StartParameter) -> Self {
        Self {
            plugins: PluginManager::new(),
            task_listeners: vec![],
            task_graph_listeners: vec![],
            version: version(),
            start_parameter: start,
            graph: Default::default(),
        }
    }

    /// Makes the execution graph available
    pub fn set_execution_graph(&mut self, graph: &ExecutionGraph) -> ProjectResult {
        self.graph
            .write()
            .set(graph.clone())
            .expect("execution graph already set");
        for listener in &mut self.task_graph_listeners {
            listener.graph_ready(graph)?;
        }
        Ok(())
    }

    /// Add a listener to the inner freight
    pub fn add_listener<T: Listener<Listened = Self>>(&mut self, listener: T) -> ProjectResult {
        listener.add_listener(self)
    }

    pub(crate) fn add_task_execution_listener<T: TaskExecutionListener + 'static>(
        &mut self,
        listener: T,
    ) -> ProjectResult {
        self.task_listeners.push(Box::new(listener));
        Ok(())
    }

    pub(crate) fn add_task_execution_graph_listener<T: TaskExecutionGraphListener + 'static>(
        &mut self,
        mut listener: T,
    ) -> ProjectResult {
        if let Some(graph) = self.graph.read().get() {
            listener.graph_ready(graph)
        } else {
            self.task_graph_listeners.push(Box::new(listener));
            Ok(())
        }
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
    fn with_assemble<F, R>(&self, func: F) -> R
    where
        F: FnOnce(&Assemble) -> R;

    /// Get the assemble instance this value is aware of as a mutable reference
    fn with_assemble_mut<F, R>(&mut self, func: F) -> R
    where
        F: FnOnce(&mut Assemble) -> R;
}

impl AssembleAware for Assemble {
    /// Gets this [`Assemble`](Assemble) instance.
    fn with_assemble<F, R>(&self, func: F) -> R
    where
        F: FnOnce(&Assemble) -> R,
    {
        (func)(self)
    }

    fn with_assemble_mut<F, R>(&mut self, func: F) -> R
    where
        F: FnOnce(&mut Assemble) -> R,
    {
        (func)(self)
    }
}

impl AssembleAware for Arc<RwLock<Assemble>> {
    fn with_assemble<F, R>(&self, func: F) -> R
    where
        F: FnOnce(&Assemble) -> R,
    {
        (func)(self.read().deref())
    }

    fn with_assemble_mut<F, R>(&mut self, func: F) -> R
    where
        F: FnOnce(&mut Assemble) -> R,
    {
        (func)(self.write().deref_mut())
    }
}

/// The start parameters define the configuration used by an assemble instance to execute a build.
///
/// Generally corresponds to the command line options for assemble.
#[derive(Debug, Clone)]
pub struct StartParameter {
    current_dir: PathBuf,
    logging: LoggingArgs,
    mode: ConsoleMode,
    project_dir: Option<PathBuf>,
    properties: HashMap<String, Option<String>>,
    builder: String,
    task_requests: Vec<String>,
    workers: usize,
    backtrace: bool,
}

impl StartParameter {
    /// Creates a new instance of a start parameter with only default settings
    pub fn new() -> Self {
        Self {
            current_dir: current_dir().expect("no valid current working directory"),
            logging: LoggingArgs::default(),
            mode: ConsoleMode::Auto,
            project_dir: None,
            properties: HashMap::new(),
            builder:"".to_string(),
            task_requests: vec![],
            workers: 0,
            backtrace: false,
        }
    }

    /// Gets the current directory of the start parameter, used to select the default project and
    /// find the settings file.
    pub fn current_dir(&self) -> &Path {
        &self.current_dir
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
    pub fn properties(&self) -> &HashMap<String, Option<String>> {
        &self.properties
    }

    /// A mutable reference to the project properties for this build
    pub fn properties_mut(&mut self) -> &mut HashMap<String, Option<String>> {
        &mut self.properties
    }

    /// The builder used by this project.
    pub fn builder(&self) -> &str {
        &self.builder
    }

    /// Gets whether the backtrace should be emitted
    pub fn backtrace(&self) -> bool {
        self.backtrace
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
    pub fn set_logging(&mut self, log_level: LoggingArgs) {
        self.logging = log_level;
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
    pub fn set_builder(&mut self, builder: &str) {
        self.builder = builder.to_string();
    }

    pub fn set_backtrace(&mut self, backtrace: bool) {
        self.backtrace = backtrace;
    }

    pub fn workers(&self) -> usize {
        self.workers
    }

    pub fn set_workers(&mut self, workers: usize) {
        self.workers = workers;
    }
    pub fn logging(&self) -> &LoggingArgs {
        &self.logging
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
