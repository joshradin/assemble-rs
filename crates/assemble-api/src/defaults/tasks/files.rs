//! Tasks that are related to files (Copy, Delete, Sync)

use crate::task::{Task, IntoTask};
use std::path::PathBuf;
use crate::defaults::tasks::DefaultTask;
use crate::exception::BuildResult;
use crate::project::Project;


#[derive(IntoTask)]
#[action(dupe_files)]
pub struct Dupe {
    #[input] from: PathBuf,
    #[output] into: PathBuf,
}

#[task_action(Dupe)]
fn dupe_files(dupe: &Dupe, project: &Project) {

}

assert_impl_all!(Dupe: IntoTask);