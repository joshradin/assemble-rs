//! Tasks that are related to files (copying, deleting, etc...)

use assemble_core::exception::BuildResult;
use assemble_core::project::Project;
use assemble_core::Task;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;


/// Copies files
#[derive(Default, Clone)]
pub struct Dupe {
    from: PathBuf,
    into: PathBuf,
}

fn dupe_files(dupe: &mut Dupe, _project: &Project) -> BuildResult {
    std::fs::copy(&dupe.from, &dupe.into)?;
    Ok(())
}

/// Deletes files
pub struct Delete {}
