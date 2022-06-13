//! Errors that occur in the daemon

use std::convert::Infallible;
use thiserror::Error;
use crate::message::RequestBufferEmpty;

/// An error occurred in the daemon
#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("{0}")]
    Custom(String),
    #[error("Could not create daemon server")]
    DaemonServerCouldNotBeCreated(lockfile::Error),
    #[error("infallible error isn't infallible")]
    Infallible(#[from] Infallible),
    #[error(transparent)]
    DevNullError(#[from] RequestBufferEmpty)
}

impl DaemonError {
    pub fn custom(message: impl AsRef<str>) -> Self {
        Self::Custom(message.as_ref().to_string())
    }
}
