//! Contains development aids for creating freight projects
//!

use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use std::path::{Path, PathBuf};

use crate::build;
use crate::builders::BuildConfigurator;
use assemble_core::prelude::StartParameter;
use itertools::Itertools;
use tempfile::tempdir;
use assemble_core::error::PayloadError;
use crate::error::AssembleError;

/// Run freight using a custom environment
pub struct FreightRunner<B: BuildConfigurator> {
    assemble_home: Box<dyn AsRef<Path>>,
    project_home: Box<dyn AsRef<Path>>,
    builder: B,
}

impl<B: BuildConfigurator> FreightRunner<B> {
    pub fn assemble_home(&self) -> &Path {
        (*self.assemble_home).as_ref()
    }

    pub fn project_home(&self) -> &Path {
        (*self.project_home).as_ref()
    }

    /// Runs the default tasks
    #[inline]
    pub fn default(&self) -> Result<(), PayloadError<AssembleError>>
    where
        B::Err: 'static,
        AssembleError: From<B::Err>
    {
        self.execute::<_, &str>([])
    }

    /// Execute the given list of args
    pub fn execute<I, S>(&self, args: I) -> Result<(), PayloadError<AssembleError>>
    where
        S: AsRef<str>,
        I: IntoIterator<Item = S>,
        B::Err: 'static,
        AssembleError: From<B::Err>
    {
        let freight = StartParameter::new().with_task_requests(args);
        match build(freight, &self.builder) {
            Ok(ok) => Ok(()),
            Err(err) => {
                return Err(err);
            }
        }
    }
}

impl<B: BuildConfigurator + Debug> Debug for FreightRunner<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FreightRunner")
            .field("assemble_home", &(self.assemble_home.deref()).as_ref())
            .field("project_home", &(self.project_home.deref()).as_ref())
            .field("builder", &self.builder)
            .finish()
    }
}

/// Build a freight runner instance
pub struct FreightRunnerBuilder<B: BuildConfigurator> {
    assemble_home: Option<PathBuf>,
    project_home: Option<PathBuf>,
    builder: B,
}

impl<B: BuildConfigurator> FreightRunnerBuilder<B> {
    pub fn new() -> Self
    where
        B: Default,
    {
        Self {
            assemble_home: None,
            project_home: None,
            builder: B::default(),
        }
    }

    pub fn with_assemble_home<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.assemble_home = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn with_project<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.project_home = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn build(self) -> FreightRunner<B> {
        FreightRunner {
            assemble_home: self
                .assemble_home
                .map(|p| Box::new(p) as Box<dyn AsRef<Path>>)
                .unwrap_or_else(|| Box::new(tempdir().unwrap()) as Box<dyn AsRef<Path>>),
            project_home: self
                .project_home
                .map(|p| Box::new(p) as Box<dyn AsRef<Path>>)
                .unwrap_or_else(|| Box::new(tempdir().unwrap()) as Box<dyn AsRef<Path>>),
            builder: self.builder,
        }
    }
}
