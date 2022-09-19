use crate::assemble_core::lazy_evaluation::ProviderExt as _;
use assemble_core::__export::{CreateTask, InitializeTask, ProjectResult, TaskIO, TaskId};
use assemble_core::exception::{BuildError, BuildException};
use assemble_core::file_collection::{FileCollection, FileSet};
use assemble_core::flow::output::SinglePathOutputTask;
use assemble_core::lazy_evaluation::{Prop, Provider};
use assemble_core::prelude::ProjectError;
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::utilities::{not, spec, Callback};
use assemble_core::{BuildResult, Executable, Project, Task};
use assemble_std::extensions::project_extensions::ProjectExec;
use log::Level;
use std::os;
use std::path::{Path, PathBuf};
use std::process::Command;
use assemble_std::specs::exec_spec::Output;

/// Compile the project.
///
/// The output path is os-dependent.
#[derive(Debug, CreateTask, TaskIO)]
pub struct CompileProject {
    #[input(files)]
    cargo_files: FileSet,
    #[output]
    lib: Prop<PathBuf>,
    pub project_dir: Prop<PathBuf>,
}

impl CompileProject {
    /// Sets the source for this project
    pub fn source(&mut self, fc: impl AsRef<Path>) {
        let files = FileSet::from(fc).filter("*.rs").filter(not("**/target/**"));
        self.cargo_files = files;
    }

    fn set_output(&mut self, target_dir: impl Provider<PathBuf> + 'static) -> ProjectResult {
        let mut built_path = target_dir.map(|p| p.join("release"));
        if cfg!(target_os = "windows") {
            self.lib
                .set_with(built_path.map(|p| p.join("build_logic.dll")))?;
        } else if cfg!(target_os = "macos") {
            self.lib
                .set_with(built_path.map(|p| p.join("build_logic.dylib")))?;
        } else if cfg!(target_os = "linux") {
            self.lib
                .set_with(built_path.map(|p| p.join("build_logic.so")))?;
        } else {
            return Err(ProjectError::custom("unsupported os for assemble").into());
        };

        Ok(())
    }
}

impl SinglePathOutputTask for CompileProject {
    fn get_path(task: &Executable<Self>) -> PathBuf {
        task.lib.get()
    }
}

impl UpToDate for CompileProject {
    fn up_to_date(&self) -> bool {
        false
    }
}

impl InitializeTask for CompileProject {
    fn initialize(task: &mut Executable<Self>, _project: &Project) -> ProjectResult {
        let target_dir = task.project_dir.clone().map(|p| p.join("target"));
        task.set_output(target_dir)?;

        Ok(())
    }
}

impl Task for CompileProject {
    fn task_action(task: &mut Executable<Self>, project: &Project) -> BuildResult {
        let files = task.cargo_files.try_files()?;
        info!("relevant files = {:#?}", files);
        info!("output lib = {:?}", task.lib.fallible_get()?);

        let project_dir = task.project_dir.fallible_get()?;
        info!("executing in project dir: {:?}", project_dir);
        // let output = Command::new("cargo")
        //     .args(["build", "--release"])
        //     .current_dir(project_dir)
        //     .output()?;
        // if !output.status.success() {
        //     Err(BuildError::new("could not build"))?;
        // }
        project
            .exec_with(|spec| {
                spec.exec("cargo")
                    .args(["build", "--release"])
                    .working_dir(project_dir)
                    .stderr(Output::Null);
            })?
            .expect_success()?;

        Ok(())
    }
}
