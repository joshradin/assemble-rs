//! Contains builders for making projects

use assemble_core::prelude::SharedProject;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

/// Simplified version of project properties
pub type ProjectProperties = HashMap<String, Option<String>>;

#[cfg(feature = "yaml")]
pub mod yaml;

/// Define a builder to make projects
pub trait Builder {
    type Err: Error;

    /// Open a project in a specific directory
    fn open<P: AsRef<Path>>(
        path: P,
        properties: &ProjectProperties,
    ) -> Result<SharedProject, Self::Err>;

    /// Attempt to find a project by searching up a directory
    fn discover<P: AsRef<Path>>(
        path: P,
        properties: &ProjectProperties,
    ) -> Result<SharedProject, Self::Err>;
}
