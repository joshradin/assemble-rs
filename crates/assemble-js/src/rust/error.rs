//! Used for better error handling

use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    SystemError(#[from] rquickjs::Error),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error("{0}")]
    UserError(String)
}