//! Errors that occur in the daemon

use thiserror::Error;

/// An error occurred in the daemon
#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("{0}")]
    Custom(String),
    #[error("Could not create daemon server")]
    DaemonServerCouldNotBeCreated(lockfile::Error),
}

impl DaemonError {
    pub fn custom(message: impl AsRef<str>) -> Self {
        Self::Custom(message.as_ref().to_string())
    }
}
