//! The wrapper task

use assemble_core::__export::{CreateTask, InitializeTask, TaskId, TaskIO};
use assemble_core::cryptography::Sha256;
use assemble_core::properties::Prop;
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::{BuildResult, Executable, Project, Task};
use std::path::PathBuf;
use url::Url;
use assemble_core::project::error::ProjectResult;

/// creates a script to download and then run the assemble distributable
#[derive(Debug, CreateTask, TaskIO)]
pub struct WrapperTask {
    distribution_base: Prop<String>,
    distribution_path: Prop<PathBuf>,
    distribution_url: Prop<Url>,
    distribution_sha256: Prop<Sha256>,
}

impl UpToDate for WrapperTask {}

impl InitializeTask for WrapperTask {}

impl Task for WrapperTask {
    fn task_action(_task: &mut Executable<Self>, _project: &Project) -> BuildResult {
        todo!()
    }
}
