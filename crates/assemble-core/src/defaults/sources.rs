//! Sources

use crate::dependencies::{Dependency, DependencyKey, DownloadError, UnresolvedDependency};
use std::collections::HashMap;
use std::path::PathBuf;

pub mod crate_registry;
pub mod local;
