//! Tasks that are related to files (copying, deleting, etc...)

use assemble_core::defaults::task::DefaultTask;
use assemble_core::exception::BuildResult;
use assemble_core::project::Project;
use assemble_core::{ExecutableTask, Task};
use std::fs::{File, OpenOptions};
use std::path::PathBuf;

use assemble_core::task_action;

/// Copies files
#[derive(Task, Default)]
#[action(dupe_files)]
pub struct Dupe {
    #[input]
    from: PathBuf,
    #[output]
    into: PathBuf,
}

#[task_action]
fn dupe_files(dupe: &mut Dupe, _project: &Project) -> BuildResult {
    std::fs::copy(&dupe.from, &dupe.into)?;
    Ok(())
}

/// Deletes files
pub struct Delete {}
