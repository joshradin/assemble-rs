use assemble_core::__export::{CreateTask, InitializeTask, ProjectResult, TaskIO, TaskId};
use assemble_core::exception::BuildError;
use assemble_core::file_collection::FileCollection;
use assemble_core::file_collection::FileSet;
use assemble_core::lazy_evaluation::Prop;
use assemble_core::lazy_evaluation::{Provider, ProviderExt};
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::{BuildResult, Executable, Project, Task};
use std::io::Write;
use std::path::PathBuf;

/// Create the `lib.rs` file
#[derive(Debug, CreateTask, TaskIO)]
#[description("Creates the lib.rs file")]
pub struct CreateLibRs {
    #[input(files)]
    pub project_script_files: FileSet,
    pub project_dir: Prop<PathBuf>,
    #[output]
    pub lib_file: Prop<PathBuf>,
}

impl UpToDate for CreateLibRs {}

impl InitializeTask for CreateLibRs {
    fn initialize(task: &mut Executable<Self>, _project: &Project) -> ProjectResult {
        let lib_file = task.project_dir.clone().map(|d| d.join("lib.rs"));
        task.lib_file.set_with(lib_file)?;
        Ok(())
    }
}

impl Task for CreateLibRs {
    fn task_action(task: &mut Executable<Self>, _project: &Project) -> BuildResult {
        info!(
            "script files = {:#?}",
            task.project_script_files.try_files()?
        );
        info!("lib file = {:?}", task.lib_file.fallible_get()?);

        let mut file = task.lib_file.create()?;

        let mut modules = vec![];

        for script in &task.project_script_files {
            let module = script
                .file_name()
                .unwrap()
                .to_string_lossy()
                .strip_suffix(".rs")
                .unwrap()
                .replace("-", "_");
            modules.push(module.clone());

            writeln!(file, "#[path = {:?}]", script)?;
            writeln!(file, "mod {};", module)?;
        }

        return Ok(());
    }
}
