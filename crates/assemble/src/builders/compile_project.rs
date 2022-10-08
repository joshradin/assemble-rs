use crate::assemble_core::lazy_evaluation::ProviderExt as _;
use assemble_core::__export::ProjectResult;

use assemble_core::file_collection::{FileCollection, FileSet};
use assemble_core::flow::output::SinglePathOutputTask;
use assemble_core::lazy_evaluation::{Prop, Provider};
use assemble_core::prelude::ProjectError;
use assemble_core::task::create_task::CreateTask;
use assemble_core::task::initialize_task::InitializeTask;

use assemble_core::task::up_to_date::UpToDate;
use assemble_core::utilities::not;
use assemble_core::{BuildResult, Executable, Project, Task};
use assemble_std::extensions::project_extensions::ProjectExec;
use assemble_std::specs::exec_spec::Output;

use std::path::{Path, PathBuf};

/// Compile the project.
///
/// The output path is os-dependent.
#[derive(Debug, CreateTask, TaskIO)]
pub struct CompileProject {
    #[input(files)]
    cargo_files: Prop<FileSet>,
    #[output(file)]
    pub lib: Prop<PathBuf>,
    pub project_dir: Prop<PathBuf>,
}

impl CompileProject {
    /// Sets the source for this project
    pub fn source(&mut self, fc: impl AsRef<Path>) {
        let files = FileSet::from(fc).filter("*.rs").filter(not("**/target/**"));
        self.cargo_files.set(files).unwrap();
    }

    fn set_output(&mut self, target_dir: impl Provider<PathBuf> + 'static) -> ProjectResult {
        let built_path = target_dir.map(|p| p.join("release"));
        if cfg!(target_os = "windows") {
            self.lib
                .set_with(built_path.map(|p| p.join("build_logic.dll")))?;
        } else if cfg!(target_os = "macos") {
            self.lib
                .set_with(built_path.map(|p| p.join("libbuild_logic.dylib")))?;
        } else if cfg!(target_os = "linux") {
            self.lib
                .set_with(built_path.map(|p| p.join("libbuild_logic.so")))?;
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
    // fn up_to_date(&self) -> bool {
    //     false
    // }
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
        let files = task.cargo_files.fallible_get()?.try_files()?;
        info!("relevant files = {:#?}", files);
        info!("output lib = {:?}", task.lib.fallible_get()?);

        let project_dir = task.project_dir.fallible_get()?;
        info!("executing in project dir: {:?}", project_dir);
        project
            .exec_with(|spec| {
                spec.exec("cargo")
                    .args(["build", "--release", "--color", "always"])
                    .add_env("RUSTFLAGS", "-Awarnings")
                    .working_dir(project_dir)
                    .stderr(Output::Null);
            })?
            .expect_success()?;

        Ok(())
    }
}
