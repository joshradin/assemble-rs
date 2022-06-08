mod daemon_server;
mod daemon;

use std::path::Path;
pub use daemon::*;
use crate::error::DaemonError;

/// Common result type for daemons
pub type DaemonResult<T> = Result<T, DaemonError>;

/// Recover the state of something from a path
trait RecoverState : Sized {
    type Err;

    /// Try recovering the state of some object from a path
    fn try_recover(path: &Path) -> Result<Self, Self::Err>;
}