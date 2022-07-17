//! The exec spec helps with defining executables

use assemble_core::project::VisitProject;
use assemble_core::task::executable::Executable;
use assemble_core::{Project, Task};
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdout, Command, ExitStatus, Stdio};

/// The exec spec helps define something to execute by the project
#[derive(Debug, Default)]
pub struct ExecSpec {
    /// The working directory to run the executable in
    working_dir: PathBuf,
    /// The executable
    executable: OsString,
    /// The command line args for the executable
    clargs: Vec<String>,
    /// The environment variables for the executable.
    ///
    /// # Warning
    /// **ONLY** the environment variables in this map will be passed to the executable.
    env: HashMap<String, String>,
    child_process: Option<Child>,
}

impl ExecSpec {
    /// The working directory of the exec spec. If the path is relative, then the relative
    /// path is calculated relative to the the base directory of a project.
    pub fn working_dir(&self) -> &Path {
        &self.working_dir
    }
    /// The executable to run
    pub fn executable(&self) -> &OsStr {
        &self.executable
    }

    /// Command line args for the exec spec
    pub fn args(&self) -> &Vec<String> {
        &self.clargs
    }

    /// The environment variables for the exec spec
    pub fn env(&self) -> &HashMap<String, String> {
        &self.env
    }

    #[doc(hidden)]
    pub(crate) fn execute(&mut self, path: impl AsRef<Path>) -> io::Result<&Child> {
        let working_dir = if self.working_dir().is_absolute() {
            self.working_dir().to_path_buf()
        } else {
            path.as_ref().join(self.working_dir()).canonicalize()?
        };

        let mut command = Command::new(self.executable());
        command.env_clear().envs(self.env());
        command.current_dir(working_dir);
        command.args(self.args());

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let mut child = command.spawn()?;

        self.child_process = Some(child);
        Ok(self.child_process.as_ref().unwrap())
    }

    /// Gets the streams of a running [child](std::process::Child) process.
    ///
    /// # Warning
    /// Will only get streams after the child process starts and before its been finished using
    /// [`finish()`](Self::finish).
    ///
    /// This will only return both streams if the [`stdout`] and [`stderr`] can be retrieved.
    /// Otherwise [`None`](None) is returned. This means that the `stdout` and `stderr` can only be
    /// retrieved once for each time the [`ExecSpec`](Self) is run
    ///
    /// [`stdout`]: ChildStdout
    /// [`stderr`]: ChildStderr
    pub fn streams(&mut self) -> Option<(ChildStdout, ChildStderr)> {
        if let Some(child) = &mut self.child_process {
            let stdout = child.stdout.take();
            let stderr = child.stderr.take();
            stdout.zip(stderr)
        } else {
            None
        }
    }

    /// Waits for the running child process to finish. Will return [`Some(exit_status)`](Some) only
    /// if a child process has already been started. Otherwise, a [`None`](None) result will be given
    pub fn finish(&mut self) -> io::Result<ExitStatus> {
        let child = std::mem::replace(&mut self.child_process, None);
        if let Some(mut child) = child {
            child.wait()
        } else {
            Err(io::Error::new(ErrorKind::Other, "No child process"))
        }
    }
}

impl Clone for ExecSpec {
    /// Creates a clone of the ExecSpec. Will not clone over the running child process, if it exists.
    fn clone(&self) -> Self {
        Self {
            working_dir: self.working_dir.clone(),
            executable: self.executable.clone(),
            clargs: self.clargs.clone(),
            env: self.env.clone(),
            child_process: None,
        }
    }
}

impl VisitProject<Result<(), io::Error>> for ExecSpec {
    /// Executes the exec spec in the project.
    fn visit(&mut self, project: &Project) -> Result<(), io::Error> {
        self.execute(project.project_dir()).map(|_| ())
    }
}

/// Builds exec specs
pub struct ExecSpecBuilder {
    /// The working directory to run the executable in
    pub working_dir: Option<PathBuf>,
    /// The executable
    pub executable: Option<OsString>,
    /// The command line args for the executable
    pub clargs: Vec<String>,
    /// The environment variables for the executable. By default, the exec spec will
    /// inherit from the parent process.
    ///
    /// # Warning
    /// **ONLY** The environment variables in this map will be passed to the executable.
    pub env: HashMap<String, String>,
}

/// An exec spec configuration error
#[derive(Debug, thiserror::Error)]
#[error("{}", error)]
pub struct ExecSpecBuilderError {
    error: String,
}

impl From<&str> for ExecSpecBuilderError {
    fn from(s: &str) -> Self {
        Self {
            error: s.to_string(),
        }
    }
}

impl ExecSpecBuilder {
    /// Create a new [ExecSpecBuilder](Self).
    pub fn new() -> Self {
        Self {
            working_dir: Some(PathBuf::new()),
            executable: None,
            clargs: vec![],
            env: Self::default_env(),
        }
    }

    /// The default environment variables
    pub fn default_env() -> HashMap<String, String> {
        std::env::vars().into_iter().collect()
    }

    /// Changes the environment variables to the contents of this map.
    ///
    /// # Warning
    /// This will clear all previously set values in the environment map
    pub fn with_env<I: IntoIterator<Item = (String, String)>>(&mut self, env: I) -> &mut Self {
        self.env = env.into_iter().collect();
        self
    }

    /// Adds variables to the environment
    pub fn extend_env<I: IntoIterator<Item = (String, String)>>(&mut self, env: I) -> &mut Self {
        self.env.extend(env);
        self
    }

    /// Add an arg to the command
    pub fn arg<S: AsRef<str>>(&mut self, arg: S) -> &mut Self {
        self.clargs.push(arg.as_ref().to_string());
        self
    }

    /// Add many args to the command
    pub fn args<I, S: AsRef<str>>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
    {
        self.clargs
            .extend(args.into_iter().map(|s| s.as_ref().to_string()));
        self
    }

    /// Set the executable for the exec spec
    pub fn exec<E: AsRef<OsStr>>(&mut self, exec: E) -> &mut Self {
        self.executable = Some(exec.as_ref().to_os_string());
        self
    }

    /// Set the working directory for the exec spec. If the path is relative, it will be
    /// resolved to the project directory.
    pub fn working_dir<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.working_dir = Some(path.as_ref().to_path_buf());
        self
    }

    /// Build the exec spec from the builder
    ///
    /// # Error
    /// Will return an error if the working directory or the executable isn't set.
    pub fn build(self) -> Result<ExecSpec, ExecSpecBuilderError> {
        Ok(ExecSpec {
            working_dir: self
                .working_dir
                .ok_or(ExecSpecBuilderError::from("Working directory not set"))?,
            executable: self
                .executable
                .ok_or(ExecSpecBuilderError::from("Executable not set"))?,
            clargs: self.clargs,
            env: self.env,
            child_process: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_exec_spec() {
        let mut builder = ExecSpecBuilder::new();
        builder.exec("echo").arg("hello, world");
        let exec = builder.build().unwrap();
    }
}
