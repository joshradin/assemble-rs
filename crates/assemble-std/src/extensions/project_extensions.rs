//! Extensions to the [`Project`](assemble_core::Project)

use crate::private::ProjectSealed;
use crate::specs::exec_spec::{ExecSpec, ExecSpecBuilder};
use assemble_core::project::VisitProject;
use assemble_core::{Executable, Project};
use std::io;
use std::process::ExitStatus;

/// Adds [`ExecSpec`](crate::specs::exec_spec::ExecSpec) related methods to projects.
pub trait ProjectExec: ProjectSealed {
    /// Configure an [`ExecSpec`](crate::specs::exec_spec::ExecSpecBuilder), then execute it.
    ///
    /// # Example
    /// ```
    /// # use assemble_core::Project;
    /// use assemble_std::ProjectExec;
    ///
    /// # let project = Project::default();
    /// let exit_status = project.exec(|exec| {
    ///     exec.exec("echo").args(&["Hello", "World"]);
    /// }).unwrap();
    /// assert!(exit_status.success());
    /// ```
    fn exec<F>(&self, config: F) -> io::Result<ExitStatus>
    where
        F: FnOnce(&mut ExecSpecBuilder);

    /// Execute an [ExecSpec](ExecSpec) without modifying it.
    fn exec_spec(&self, exec_spec: ExecSpec) -> io::Result<ExitStatus>;
}

impl ProjectExec for Project {
    fn exec<F>(&self, config: F) -> io::Result<ExitStatus>
    where
        F: FnOnce(&mut ExecSpecBuilder),
    {
        let mut builder = ExecSpecBuilder::new();
        config(&mut builder);
        let exec_spec = builder.build().unwrap();
        self.exec_spec(exec_spec)
    }

    fn exec_spec(&self, mut exec_spec: ExecSpec) -> io::Result<ExitStatus> {
        exec_spec.visit(self)?;

        exec_spec.finish()
    }
}
