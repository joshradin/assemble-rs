//! Extensions to the [`Project`](assemble_core::Project)

use crate::private::ProjectSealed;
use crate::specs::exec_spec::{ExecSpec, ExecSpecBuilder};
use assemble_core::project::VisitProject;
use assemble_core::Project;
use colored::Colorize;
use std::io;
use std::io::{BufRead, BufReader, Read};
use std::process::ExitStatus;

/// Adds [`ExecSpec`](crate::specs::exec_spec::ExecSpec) related methods to projects.
pub trait ProjectExec: ProjectSealed {
    /// Automatically executes a spec and logs output streams
    fn exec<F>(&self, config: F) -> io::Result<ExitStatus>
    where
        F: FnOnce(&mut ExecSpecBuilder);

    /// Configure an [`ExecSpec`](crate::specs::exec_spec::ExecSpecBuilder), then execute it.
    ///
    /// # Example
    /// ```
    /// # use assemble_core::Project;
    /// use assemble_std::ProjectExec;
    ///
    /// # let project = Project::temp(None);
    /// let exit_status = project.with(|p| p.exec_with(|exec| {
    ///     exec.exec("echo").args(&["Hello", "World"]);
    /// })).unwrap();
    /// assert!(exit_status.0.success());
    /// ```
    fn exec_with<F>(&self, config: F) -> io::Result<(ExitStatus, Option<(Vec<u8>, Vec<u8>)>)>
    where
        F: FnOnce(&mut ExecSpecBuilder);

    /// Execute an [ExecSpec](ExecSpec) without modifying it.
    fn exec_spec(
        &self,
        exec_spec: ExecSpec,
    ) -> io::Result<(ExitStatus, Option<(Vec<u8>, Vec<u8>)>)>;
}

impl ProjectExec for Project {
    fn exec<F>(&self, config: F) -> io::Result<ExitStatus>
    where
        F: FnOnce(&mut ExecSpecBuilder),
    {
        let mut builder = ExecSpecBuilder::new();
        config(&mut builder);
        let mut exec_spec = builder.build().unwrap();

        exec_spec.visit(self)?;
        if let Some((out, err)) = exec_spec.streams() {
            let mut out_lines = BufReader::new(out);
            let mut err_lines = BufReader::new(err);
            loop {
                let mut buffer = String::new();
                let out_read = out_lines.read_line(&mut buffer)?;
                if !buffer.trim_end().is_empty() {
                    info!("{}", buffer.trim_end());
                }
                let mut buffer = String::new();
                let err_read = err_lines.read_line(&mut buffer)?;
                if !buffer.trim_end().is_empty() {
                    info!("{}", buffer.trim_end().red());
                }
                if out_read == 0 && err_read == 0 {
                    break;
                }
            }
            let status = exec_spec.finish()?;
            Ok(status)
        } else {
            exec_spec.finish()
        }
    }

    fn exec_with<F>(&self, config: F) -> io::Result<(ExitStatus, Option<(Vec<u8>, Vec<u8>)>)>
    where
        F: FnOnce(&mut ExecSpecBuilder),
    {
        let mut builder = ExecSpecBuilder::new();
        config(&mut builder);
        let exec_spec = builder.build().unwrap();
        self.exec_spec(exec_spec)
    }

    fn exec_spec(
        &self,
        mut exec_spec: ExecSpec,
    ) -> io::Result<(ExitStatus, Option<(Vec<u8>, Vec<u8>)>)> {
        exec_spec.visit(self)?;
        if let Some((out, err)) = exec_spec.streams() {
            let status = exec_spec.finish()?;
            let out_bytes = out.bytes();
            let err_bytes = err.bytes();
            Ok((
                status,
                Some((
                    out_bytes.collect::<Result<Vec<_>, _>>()?,
                    err_bytes.collect::<Result<Vec<_>, _>>()?,
                )),
            ))
        } else {
            exec_spec.finish().map(|e| (e, None))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ProjectExec;
    use assemble_core::Project;

    #[test]
    fn hello_world() {
        let project = Project::temp(None);
        let exit_status = project
            .with(|p| {
                p.exec_with(|exec| {
                    exec.exec("echo").args(&["Hello", "World"]);
                })
            })
            .unwrap();
        assert!(exit_status.0.success());
    }
}
