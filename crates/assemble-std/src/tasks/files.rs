//! Tasks that are related to files (copying, deleting, etc...)

use assemble_core::exception::{BuildException, BuildResult};
use assemble_core::project::Project;

use std::path::PathBuf;
use assemble_core::error::PayloadError;

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
