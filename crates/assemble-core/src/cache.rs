//! The cache used assemble wise. This is accessible from every project, and should be used with care

use std::env;
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
        let home =
            env::var("USER_HOME")
                .map(PathBuf::from)
                .ok()
                .or(dirs::home_dir())
                .or(env::current_dir().ok())
                .expect("No USER_HOME, HOME, or current directory to place cache. (how are you even running this?)");

        let path = home.join(".assemble");
        Self { path }
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
