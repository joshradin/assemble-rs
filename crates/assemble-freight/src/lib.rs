#![deny(rustdoc::broken_intra_doc_links)]

//! Freight is the main implementation library for how assemble projects are built.
//!
//! Binaries produced by the bin maker should use this library for execution purposes.

use std::sync::{Arc, RwLock};
use crate::core::{ExecutionGraph, ExecutionPlan, TaskResolver};
use assemble_core::plugins::{PluginAware, PluginManager};
use assemble_core::prelude::SharedProject;
use assemble_core::project::ProjectError;
use assemble_core::version::{Version, version};

use crate::project_properties::ProjectProperties;
use crate::utils::{FreightError, FreightResult, TaskResult, TaskResultBuilder};

#[macro_use]
extern crate log;

pub mod cli;
pub mod core;
pub mod listeners;
pub mod ops;
pub mod project_properties;
pub mod utils;

pub use crate::cli::FreightArgs;
use crate::listeners::{Listener, TaskExecutionListener};

#[derive(Clone)]
pub struct Freight(Arc<RwLock<FreightInner>>);

/// new way to access freight. Can attach listeners to to this.
struct FreightInner {
    args: FreightArgs,
    version: Version,
    plugin_manager: PluginManager<FreightInner>,
    task_listeners: Vec<Box<dyn TaskExecutionListener>>,
    project: Option<SharedProject>,
}

impl FreightInner {
    pub fn new(args: FreightArgs) -> Self {
        Self {
            args,
            version: version(),
            plugin_manager: PluginManager::default(),
            task_listeners: vec![],
            project: None,
        }
    }

    pub(crate) fn set_project(&mut self, project: &SharedProject) {
        self.project.replace(project.clone());
    }

    pub fn add_listener<T: Listener>(&mut self, listener: T) {
        listener.add_listener(self)
    }
    pub fn add_task_execution_listener<T: TaskExecutionListener + 'static>(&mut self, listener: T) {
        self.task_listeners.push(Box::new(listener))
    }

    pub fn args(&self) -> &FreightArgs {
        &self.args
    }

    pub fn version(&self) -> &Version {
        &self.version
    }

    pub fn execute(&self) -> FreightResult<Vec<TaskResult>> {
        let ref project = self
            .project
            .as_ref()
            .ok_or(ProjectError::custom("no root project set"))?
            .clone();
        let exec_graph = {
            let resolver = TaskResolver::new(project);
            let task_requests = self.args.task_requests(project)?;
            resolver.to_execution_graph(task_requests)?
        };

        ops::execute_tasks(exec_graph, self, project)
    }
    /// Sets the args and returns the previous value
    pub fn set_args(&mut self, args: FreightArgs) -> FreightArgs {
        std::mem::replace(&mut self.args, args)

    }
}

impl PluginAware for FreightInner {
    fn plugin_manager(&self) -> PluginManager<Self> {
        self.plugin_manager.clone()
    }
}
