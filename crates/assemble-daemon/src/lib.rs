mod daemon;
pub mod daemon_server;

use crate::error::DaemonError;
pub use daemon::*;
use std::path::Path;

/// Common result type for daemons
pub type DaemonResult<T> = Result<T, DaemonError>;

/// Recover the state of something from a path
trait RecoverState: Sized {
    type Err;

    /// Try recovering the state of some object from a path
    fn try_recover(path: &Path) -> Result<Self, Self::Err>;
}

pub fn launch_daemon() {}
