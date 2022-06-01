//! Tasks that are related to files (copying, deleting, etc...)

use assemble_api::defaults::task::DefaultTask;
use assemble_api::exception::BuildResult;
use assemble_api::project::Project;
use assemble_api::task::{IntoTask, Task};
use std::path::PathBuf;

#[derive(IntoTask)]
#[action(dupe_files)]
pub struct Dupe {
    #[input]
    from: PathBuf,
    #[output]
    into: PathBuf,
}

#[task_action]
fn dupe_files(dupe: &dyn Task, project: &Project) -> BuildResult {
    todo!()
}

fn dupe_files(dupe: &mut Dupe, project: &Project) -> BuildResult {
    todo!()
}

// assert_impl_all!(Dupe: IntoTask);
