//! The exec spec helps with defining executables

use assemble_core::exception::{BuildError, BuildException};
use assemble_core::logging::{Origin, LOGGING_CONTROL};
use assemble_core::prelude::{ProjectError, ProjectResult};
use assemble_core::project::VisitProject;
use assemble_core::{BuildResult, Project, Task};
use log::Level;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, ErrorKind, Read, Stdin, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdout, Command, ExitCode, ExitStatus, Stdio};
use std::str::{Bytes, Utf8Error};
use std::string::FromUtf8Error;
use std::sync::{Arc, RwLock};
use std::thread::JoinHandle;
use std::{io, thread};

/// Input for exec
#[derive(Debug, Default, Clone)]
pub enum Input {
    /// No input
    #[default]
    Null,
    /// Get input bytes from a file
    File(PathBuf),
    /// Get input bytes from a byte vector
    Bytes(Vec<u8>),
}

impl From<&[u8]> for Input {
    fn from(b: &[u8]) -> Self {
        Self::Bytes(b.into_iter().map(|s| *s).collect())
    }
}

impl From<Vec<u8>> for Input {
    fn from(c: Vec<u8>) -> Self {
        Self::Bytes(c)
    }
}

impl<'a> From<Bytes<'a>> for Input {
    fn from(b: Bytes<'a>) -> Self {
        Self::Bytes(b.collect())
    }
}

impl From<String> for Input {
    fn from(str: String) -> Self {
        Self::from(str.bytes())
    }
}

impl From<&str> for Input {
    fn from(str: &str) -> Self {
        Self::from(str.bytes())
    }
}

impl From<&Path> for Input {
    fn from(p: &Path) -> Self {
        Self::File(p.to_path_buf())
    }
}

impl From<PathBuf> for Input {
    fn from(file: PathBuf) -> Self {
        Self::File(file)
    }
}

/// Output types for exec
#[derive(Debug, Clone)]
pub enum Output {
    /// Throw the output away
    Null,
    /// Stream the output into a file
    ///
    /// If append is true, then a new file isn't created if one at the path
    /// already exists. and text is appended. Otherwise a new file
    /// is created, replacing any old file.
    File {
        /// The path of the file to emit output to
        path: PathBuf,
        /// whether to append to the file or not
        append: bool,
    },
    /// Stream the output into the logger at a given level
    Log(#[doc("The log level to emit output to")] Level),
    /// Stream the output into a byte vector
    Bytes,
}

impl Output {
    /// Create a new output with a file as the target
    pub fn new<P: AsRef<Path>>(path: P, append: bool) -> Self {
        Self::File {
            path: path.as_ref().to_path_buf(),
            append,
        }
    }
}

impl From<Level> for Output {
    fn from(lvl: Level) -> Self {
        Output::Log(lvl)
    }
}

impl From<&Path> for Output {
    fn from(path: &Path) -> Self {
        Self::File {
            path: path.to_path_buf(),
            append: false,
        }
    }
}

impl From<PathBuf> for Output {
    fn from(path: PathBuf) -> Self {
        Self::File {
            path,
            append: false,
        }
    }
}

impl Default for Output {
    fn default() -> Self {
        Self::Log(Level::Info)
    }
}

/// The exec spec helps define something to execute by the project
#[derive(Debug, Default, Clone)]
pub struct ExecSpec {
    /// The working directory to run the executable in
    pub working_dir: PathBuf,
    /// The executable
    pub executable: OsString,
    /// The command line args for the executable
    pub clargs: Vec<OsString>,
    /// The environment variables for the executable.
    ///
    /// # Warning
    /// **ONLY** the environment variables in this map will be passed to the executable.
    pub env: HashMap<String, String>,
    /// The input to the program, if needed
    pub input: Input,
    /// Where the program's stdout is emitted
    pub output: Output,
    /// Where the program's stderr is emitted
    pub output_err: Output,
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
    pub fn args(&self) -> &[OsString] {
        &self.clargs[..]
    }

    /// The environment variables for the exec spec
    pub fn env(&self) -> &HashMap<String, String> {
        &self.env
    }

    /// Try to executes an exec-spec, using the given path to resolve the current directory. If creating the program is successful, returns an
    /// [`ExecSpecHandle`](ExecSpecHandle). This is a non-blocking method, as the actual
    /// command is ran in a separate thread.
    ///
    /// Execution of the spec begins as soon as this method is called. However, all
    /// scheduling is controlled by the OS.
    ///
    /// # Error
    /// This method will return an error if the given path can not be canonicalized into an
    /// absolute path, or the executable specified by this spec does not exist.
    pub fn execute_spec<P>(self, path: P) -> ProjectResult<ExecHandle>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        let working_dir = self.resolve_working_dir(path);
        let origin = LOGGING_CONTROL.get_origin();
        ExecHandle::create(self, &working_dir, origin)
    }

    /// Resolve a working directory
    fn resolve_working_dir(&self, path: &Path) -> PathBuf {
        if self.working_dir().is_absolute() {
            self.working_dir.to_path_buf()
        } else {
            path.join(&self.working_dir)
        }
    }

    #[doc(hidden)]
    #[deprecated]
    pub(crate) fn execute(&mut self, path: impl AsRef<Path>) -> io::Result<&Child> {
        panic!("unimplemented")
    }

    /// Waits for the running child process to finish. Will return [`Some(exit_status)`](Some) only
    /// if a child process has already been started. Otherwise, a [`None`](None) result will be given
    #[deprecated]
    pub fn finish(&mut self) -> io::Result<ExitStatus> {
        panic!("unimplemented")
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
    pub clargs: Vec<OsString>,
    /// The environment variables for the executable. By default, the exec spec will
    /// inherit from the parent process.
    ///
    /// # Warning
    /// **ONLY** The environment variables in this map will be passed to the executable.
    pub env: HashMap<String, String>,
    /// The stdin for the program. null by default.
    stdin: Input,
    output: Output,
    output_err: Output,
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
            stdin: Input::default(),
            output: Output::default(),
            output_err: Output::Log(Level::Warn),
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
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.clargs.push(arg.as_ref().to_os_string());
        self
    }

    /// Add many args to the command
    pub fn args<I, S: AsRef<OsStr>>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
    {
        self.clargs
            .extend(args.into_iter().map(|s| s.as_ref().to_os_string()));
        self
    }

    /// Add an arg to the command
    pub fn with_arg<S: AsRef<OsStr>>(mut self, arg: S) -> Self {
        self.arg(arg);
        self
    }

    /// Add many args to the command
    pub fn with_args<I, S: AsRef<OsStr>>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
    {
        self.args(args);
        self
    }

    /// Set the executable for the exec spec
    pub fn exec<E: AsRef<OsStr>>(&mut self, exec: E) -> &mut Self {
        self.executable = Some(exec.as_ref().to_os_string());
        self
    }

    /// Set the executable for the exec spec
    pub fn with_exec<E: AsRef<OsStr>>(mut self, exec: E) -> Self {
        self.exec(exec);
        self
    }

    /// Set the working directory for the exec spec. If the path is relative, it will be
    /// resolved to the project directory.
    pub fn working_dir<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.working_dir = Some(path.as_ref().to_path_buf());
        self
    }

    /// Set the standard input for the executable. doesn't need to be set
    pub fn stdin<In>(&mut self, input: In) -> &mut Self
    where
        In: Into<Input>,
    {
        let input = input.into();
        self.stdin = input;
        self
    }

    /// Set the standard input for the executable. doesn't need to be set
    pub fn with_stdin<In>(mut self, input: In) -> Self
    where
        In: Into<Input>,
    {
        self.stdin(input);
        self
    }

    /// Sets the output type for this exec spec
    pub fn stdout<O>(&mut self, output: O) -> &mut Self
    where
        O: Into<Output>,
    {
        self.output = output.into();
        self
    }

    /// Sets the output type for this exec spec
    pub fn with_stdout<O>(mut self, output: O) -> Self
    where
        O: Into<Output>,
    {
        self.stdout(output);
        self
    }

    /// Sets the output type for this exec spec
    pub fn stderr<O>(&mut self, output: O) -> &mut Self
    where
        O: Into<Output>,
    {
        self.output_err = output.into();
        self
    }

    /// Sets the output type for this exec spec
    pub fn with_stderr<O>(mut self, output: O) -> Self
    where
        O: Into<Output>,
    {
        self.stderr(output);
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
            input: self.stdin,
            output: self.output,
            output_err: self.output_err,
        })
    }
}

/// A handle into an exec spec. Can be queried to get output.
pub struct ExecHandle {
    spec: ExecSpec,
    output: Arc<RwLock<ExecSpecOutputHandle>>,
    handle: JoinHandle<io::Result<ExitStatus>>,
}

impl ExecHandle {
    fn create(spec: ExecSpec, working_dir: &Path, origin: Origin) -> ProjectResult<Self> {
        let mut command = Command::new(&spec.executable);
        command.current_dir(working_dir).env_clear().envs(&spec.env);
        command.args(spec.args());

        let input = match &spec.input {
            Input::Null => Stdio::null(),
            Input::File(file) => {
                let file = File::open(file)?;
                Stdio::from(file)
            }
            Input::Bytes(b) => {
                let mut file = tempfile::tempfile()?;
                file.write_all(&b[..])?;
                Stdio::from(file)
            }
        };
        command.stdin(input);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let realized_output = RealizedOutput::try_from(spec.output.clone())?;
        let realized_output_err = RealizedOutput::try_from(spec.output.clone())?;

        let output_handle = Arc::new(RwLock::new(ExecSpecOutputHandle {
            origin,
            realized_output: Arc::new(RwLock::new(BufWriter::new(realized_output))),
            realized_output_err: Arc::new(RwLock::new(BufWriter::new(realized_output_err))),
        }));

        let join_handle = execute(command, &output_handle)?;

        Ok(Self {
            spec,
            output: output_handle,
            handle: join_handle,
        })
    }

    /// Wait for the exec spec handle to finish
    pub fn wait(self) -> ProjectResult<ExecResult> {
        let result = self
            .handle
            .join()
            .map_err(|_| ProjectError::custom("Couldn't join thread"))??;
        let output = self.output.read()?;
        let bytes = output.bytes();
        let bytes_err = output.bytes_err();
        Ok(ExecResult {
            code: result,
            bytes,
            bytes_err,
        })
    }
}

fn execute(
    mut command: Command,
    output: &Arc<RwLock<ExecSpecOutputHandle>>,
) -> ProjectResult<JoinHandle<io::Result<ExitStatus>>> {
    trace!("attempting to execute command: {:?}", command);
    trace!("working_dir: {:?}", command.get_current_dir());
    trace!(
        "env: {:#?}",
        command
            .get_envs()
            .into_iter()
            .map(|(key, val): (&OsStr, Option<&OsStr>)| ((
                key.to_string_lossy().to_string(),
                val.map(|v| v.to_string_lossy().to_string())
                    .unwrap_or_default()
            )))
            .collect::<HashMap<_, _>>()
    );

    let spawned = command.spawn()?;
    let output = output.clone();
    Ok(thread::spawn(move || {
        let mut spawned = spawned;
        let mut output = output;
        let origin = output.read().unwrap().origin.clone();

        let mut output_handle = output.write().expect("couldn't get output");
        let out = thread::scope(|scope| {
            let mut stdout = spawned.stdout.take().unwrap();
            let mut stderr = spawned.stderr.take().unwrap();

            let output = output_handle.realized_output.clone();
            let output_err = output_handle.realized_output_err.clone();

            let origin1 = origin.clone();
            let out_join = scope.spawn(move || -> io::Result<u64> {
                LOGGING_CONTROL.with_origin(origin1, || {
                    let mut output = output.write().expect("couldnt get output");
                    io::copy(&mut stdout, &mut *output)
                })
            });
            let err_join = scope.spawn(move || -> io::Result<u64> {
                LOGGING_CONTROL.with_origin(origin, || {
                    let mut output = output_err.write().expect("couldnt get output");
                    io::copy(&mut stderr, &mut *output)
                })
            });

            let out = spawned.wait()?;
            out_join.join().map_err(|_| {
                io::Error::new(ErrorKind::Interrupted, "emitting to output failed")
            })??;
            err_join.join().map_err(|_| {
                io::Error::new(ErrorKind::Interrupted, "emitting to error failed")
            })??;
            Ok(out)
        });

        out
    }))
}

struct ExecSpecOutputHandle {
    origin: Origin,
    realized_output: Arc<RwLock<BufWriter<RealizedOutput>>>,
    realized_output_err: Arc<RwLock<BufWriter<RealizedOutput>>>,
}

impl ExecSpecOutputHandle {
    /// Gets the bytes output if output mode is byte vector
    pub fn bytes(&self) -> Option<Vec<u8>> {
        if let RealizedOutput::Bytes(vec) = self.realized_output.read().unwrap().get_ref() {
            Some(vec.clone())
        } else {
            None
        }
    }

    /// Gets the bytes output if output mode is byte vector
    pub fn bytes_err(&self) -> Option<Vec<u8>> {
        if let RealizedOutput::Bytes(vec) = self.realized_output_err.read().unwrap().get_ref() {
            Some(vec.clone())
        } else {
            None
        }
    }
}

impl Write for ExecSpecOutputHandle {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        LOGGING_CONTROL.with_origin(self.origin.clone(), || {
            self.realized_output.write().unwrap().write(buf)
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        LOGGING_CONTROL.with_origin(self.origin.clone(), || {
            self.realized_output.write().unwrap().flush()
        })
    }
}

impl TryFrom<Output> for RealizedOutput {
    type Error = io::Error;

    fn try_from(value: Output) -> Result<Self, Self::Error> {
        match value {
            Output::Null => Ok(Self::Null),
            Output::File { path, append } => {
                let file = File::options()
                    .create(true)
                    .write(true)
                    .append(append)
                    .open(path)?;

                Ok(Self::File(file))
            }
            Output::Log(log) => Ok(Self::Log(log)),
            Output::Bytes => Ok(Self::Bytes(vec![])),
        }
    }
}

enum RealizedOutput {
    Null,
    File(File),
    Log(Level),
    Bytes(Vec<u8>),
}

impl Write for RealizedOutput {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            RealizedOutput::Null => Ok(buf.len()),
            RealizedOutput::File(f) => f.write(buf),
            RealizedOutput::Log(l) => {
                log!(*l, "{}", String::from_utf8_lossy(buf));
                Ok(buf.len())
            }
            RealizedOutput::Bytes(b) => {
                b.extend(buf);
                Ok(buf.len())
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        if let RealizedOutput::File(file) = self {
            file.flush()
        } else {
            Ok(())
        }
    }
}

/// Gets the result of the exec spec
pub struct ExecResult {
    code: ExitStatus,
    bytes: Option<Vec<u8>>,
    bytes_err: Option<Vec<u8>>,
}

impl ExecResult {
    /// Gets the exit code for the exec spec
    pub fn code(&self) -> ExitStatus {
        self.code
    }

    /// Gets whether the exec spec is a success
    pub fn success(&self) -> bool {
        self.code.success()
    }

    /// Make this an error if exit code is not success
    pub fn expect_success(self) -> BuildResult<Self> {
        if !self.success() {
            Err(BuildException::new("expected a successful return code").into())
        } else {
            Ok(self)
        }
    }

    /// Gets the output, in bytes, if the original exec spec specified the bytes
    /// output type
    pub fn bytes(&self) -> Option<&[u8]> {
        self.bytes.as_ref().map(|s| &s[..])
    }

    /// Try to convert the output bytes into a string
    pub fn utf8_string(&self) -> Option<Result<String, FromUtf8Error>> {
        self.bytes()
            .map(|s| Vec::from_iter(s.into_iter().map(|b| *b)))
            .map(|s| String::from_utf8(s))
    }

    /// Gets the output, in bytes, if the original exec spec specified the bytes
    /// output type
    pub fn bytes_err(&self) -> Option<&[u8]> {
        self.bytes_err.as_ref().map(|s| &s[..])
    }

    /// Try to convert the output bytes into a string
    pub fn utf8_string_err(&self) -> Option<Result<String, FromUtf8Error>> {
        self.bytes_err()
            .map(|s| Vec::from_iter(s.into_iter().map(|b| *b)))
            .map(|s| String::from_utf8(s))
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
        assert_eq!(exec.executable, "echo");
    }

    #[test]
    fn can_execute_spec() {
        let spec = ExecSpecBuilder::new()
            .with_exec("echo")
            .with_args(["hello", "world"])
            .with_stdout(Output::Bytes)
            .build()
            .expect("Couldn't build exec spec");

        let result = { spec }.execute_spec("/").expect("Couldn't create handle");
        let wait = result.wait().expect("couldn't finish exec spec");
        let bytes = String::from_utf8(wait.bytes.unwrap()).unwrap();
        assert_eq!("hello world", bytes.trim());
    }

    #[test]
    fn invalid_exec_can_be_detected() {
        let spec = ExecSpecBuilder::new()
            .with_exec("please-dont-exist")
            .with_stdout(Output::Null)
            .build()
            .expect("couldn't build");

        let spawn = spec.execute_spec("/");

        assert!(matches!(spawn, Err(_)), "Should return an error");
    }

    #[test]
    fn emit_to_log() {}
}
