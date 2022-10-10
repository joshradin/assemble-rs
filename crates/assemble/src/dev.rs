//! Contains development aids for creating freight projects
//!

use crate::{build, execute_v2, FreightArgs, YamlBuilder};

use itertools::Itertools;

use std::path::{Path, PathBuf};

use tempfile::tempdir;

/// Run freight using a custom environment
pub struct FreightRunner {
    assemble_home: Box<dyn AsRef<Path>>,
    project_home: Box<dyn AsRef<Path>>,
}

impl FreightRunner {
    pub fn assemble_home(&self) -> &Path {
        (*self.assemble_home).as_ref()
    }

    pub fn project_home(&self) -> &Path {
        (*self.project_home).as_ref()
    }

    // /// Runs the default tasks
    // #[inline]
    // pub fn default(&self) -> anyhow::Result<()> {
    //     self.execute::<_, &str>([])
    // }
    //
    // /// Execute the given list of args
    // pub fn execute<I, S>(&self, args: I) -> anyhow::Result<()>
    // where
    //     S: AsRef<str>,
    //     I: IntoIterator<Item = S>,
    // {
    //     let args: String = Itertools::intersperse(
    //         args.into_iter().map(|s: S| format!("{:?}", s.as_ref())),
    //         " ".to_string(),
    //     )
    //     .collect();
    //     let freight = FreightArgs::command_line(args);
    //     build(freight.into(), YamlBuilder)
    // }
}

/// Build a freight runner instance
pub struct FreightRunnerBuilder {
    assemble_home: Option<PathBuf>,
    project_home: Option<PathBuf>,
}

impl FreightRunnerBuilder {
    pub fn new() -> Self {
        Self {
            assemble_home: None,
            project_home: None,
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

    pub fn build(self) -> FreightRunner {
        FreightRunner {
            assemble_home: self
                .assemble_home
                .map(|p| Box::new(p) as Box<dyn AsRef<Path>>)
                .unwrap_or_else(|| Box::new(tempdir().unwrap()) as Box<dyn AsRef<Path>>),
            project_home: self
                .project_home
                .map(|p| Box::new(p) as Box<dyn AsRef<Path>>)
                .unwrap_or_else(|| Box::new(tempdir().unwrap()) as Box<dyn AsRef<Path>>),
        }
    }
}
