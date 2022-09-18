//! Extensions to the [`Project`](assemble_core::Project)

use crate::private::ProjectSealed;
use crate::specs::exec_spec::{ExecHandle, ExecSpec, ExecSpecBuilder};
use assemble_core::prelude::ProjectResult;
use assemble_core::project::{ProjectError, VisitProject};
use assemble_core::Project;
use colored::Colorize;
use std::io;
use std::io::{BufRead, BufReader, Read};
use std::process::ExitStatus;

/// Adds [`ExecSpec`](crate::specs::exec_spec::ExecSpec) related methods to projects.
pub trait ProjectExec: ProjectSealed {
    /// Automatically executes a spec and logs output streams
    fn exec_with<F>(&self, config: F) -> ProjectResult<ExecHandle>
    where
        F: FnOnce(&mut ExecSpecBuilder),
    {
        let mut builder = self.builder();
        config(&mut builder);
        let build = builder.build().map_err(|e| ProjectError::custom(e))?;
        self.exec(build)
    }

    /// Execute something that can be made into an [`ExecSpec`](ExecSpec)
    fn exec<E>(&self, spec: E) -> ProjectResult<ExecHandle>
    where
        E: Into<ExecSpec>;

    /// Create a new builder
    fn builder(&self) -> ExecSpecBuilder {
        ExecSpecBuilder::new()
    }
}

impl ProjectExec for Project {
    fn exec<E>(&self, spec: E) -> ProjectResult<ExecHandle>
    where
        E: Into<ExecSpec>,
    {
        let path = self.project_dir();
        let exec = spec.into();
        exec.execute_spec(path)
            .map_err(|e| ProjectError::custom(e).into())
    }
}

#[cfg(test)]
mod test {
    use std::fs;
    use log::LevelFilter;
    use assemble_core::logging::{LoggingArgs, OutputType};
    use crate::ProjectExec;
    use assemble_core::Project;

    #[test]
    fn hello_world() {
        LoggingArgs::init_root_logger_with(
            LevelFilter::Trace,
            OutputType::Basic,
        );
        let project = Project::temp(None);
        project.with(|p| fs::create_dir(p.project_dir())).unwrap();
        let exit_status = project
            .with(|p| {
                p.exec_with(|exec| {
                    exec.exec("echo").args(&["Hello", "World"]);
                })
            })
            .and_then(|e| e.wait());
        if let Err(e) = &exit_status {
            println!("{}", e);
            panic!("{}", e.kind())
        }
    }
}
