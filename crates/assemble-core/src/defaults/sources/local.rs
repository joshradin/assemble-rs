//! A local dependency, referenced using a path

use crate::dependencies::Dependency;
use std::path::{Path, PathBuf};
use url::Url;

/// The local dependency struct
pub struct LocalDependency {
    path: PathBuf,
}

impl LocalDependency {
    /// Create a new local dependency from a path
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let buf = path.as_ref().to_path_buf();
        Self { path: buf }
    }
}

impl Dependency for LocalDependency {
    fn id(&self) -> &str {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap()
    }

    fn source(&self) -> Url {
        Url::from_file_path(&self.path).unwrap()
    }
}
