use crate::assemble_core::properties::ProvidesExt as _;
use assemble_core::__export::{CreateTask, InitializeTask, ProjectResult, TaskIO, TaskId};
use assemble_core::file_collection::{FileCollection, FileSet};
use assemble_core::flow::output::SinglePathOutputTask;
use assemble_core::prelude::ProjectError;
use assemble_core::properties::{Prop, Provides};
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::utilities::{not, spec, Callback};
use assemble_core::{BuildResult, Executable, Project, Task};
use assemble_std::extensions::project_extensions::ProjectExec;
use std::os;
use std::path::{Path, PathBuf};

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

    fn set_output(&mut self, target_dir: impl Provides<PathBuf> + 'static) -> ProjectResult {
        let mut built_path = target_dir.map(|p| p.join("release"));
        let path = if cfg!(windows) {
            built_path.map(|p| p.join("build_logic.dll"))
        } else {
            return Err(ProjectError::custom("unsupported os for assemble"));
        };

        self.lib.set_with(path)?;

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

        project.exec_with(|spec| {
            spec.exec("cargo")
                .args(["build", "--release"])
                .working_dir(project_dir);
        })?;

        Ok(())
    }
}
