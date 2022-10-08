//! The cache used assemble wise. This is accessible from every project, and should be used with care

use crate::ASSEMBLE_HOME;

use std::ffi::OsStr;
use std::ops::Deref;
use std::path::{Path, PathBuf};

/// The assemble cache
pub struct AssembleCache {
    path: PathBuf,
}

impl Default for AssembleCache {
    /// Creates the assemble cache at `$USER_HOME/.assemble`, `$HOME/.assemble`, then `~/.assemble`
    /// if the prior is unavailable
    fn default() -> Self {
        Self {
            path: ASSEMBLE_HOME.path().join("cache"),
        }
    }
}

impl AsRef<Path> for AssembleCache {
    fn as_ref(&self) -> &Path {
        self.path.as_path()
    }
}

impl AsRef<OsStr> for AssembleCache {
    fn as_ref(&self) -> &OsStr {
        self.path.as_os_str()
    }
}

impl Deref for AssembleCache {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
