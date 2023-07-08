//! Tasks that are related to files (copying, deleting, etc...)

use assemble_core::exception::{BuildException, BuildResult};
use assemble_core::project::Project;

use assemble_core::error::PayloadError;
use std::path::PathBuf;

/// Copies files
#[derive(Default, Clone)]
pub struct Dupe {
    from: PathBuf,
    into: PathBuf,
}

fn dupe_files(dupe: &mut Dupe, _project: &Project) -> BuildResult {
    std::fs::copy(&dupe.from, &dupe.into).map_err(PayloadError::<BuildException>::new)?;
    Ok(())
}

/// Deletes files
pub struct Delete {}
