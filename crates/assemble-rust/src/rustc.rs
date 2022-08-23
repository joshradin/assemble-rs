//! Tasks that execute through rustc

use assemble_core::dependencies::configurations::Configuration;
use assemble_core::file_collection::FileSet;
use assemble_core::{BuildResult, Executable, Project, Task};
use assemble_core::__export::{CreateTask, InitializeTask, ProjectResult, TaskId, TaskIO};
use assemble_core::task::up_to_date::UpToDate;
use assemble_std::specs::exec_spec::{ExecSpec, ExecSpecBuilder};

/// A generic rustc task
#[derive(Debug, CreateTask, TaskIO)]
pub struct RustC {
    source: FileSet,
    dependencies: FileSet
}

impl RustC {


    pub fn create_exec_spec(&self) -> ExecSpec {
        // ExecSpecBuilder::new()
        //     .exec("rustc")
        //     .build()
        //     .unwrap()
        todo!()
    }
}

impl UpToDate for RustC {}

impl InitializeTask for RustC {}

impl Task for RustC {
    fn task_action(_task: &mut Executable<Self>, _project: &Project) -> BuildResult {
        todo!()
    }
}