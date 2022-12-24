//! Error result

use std::convert::Infallible;
use assemble_core::error::PayloadError;
use assemble_core::exception::BuildException;
use assemble_core::project::ProjectError;
use crate::builders::BuildConfigurator;
use assemble_freight::utils::FreightError;
use crate::builders::js::error::JavascriptError;

#[derive(Debug, thiserror::Error)]
pub enum AssembleError {
    #[error(transparent)]
    FreightError(#[from] FreightError),
    #[error(transparent)]
    ProjectError(#[from] ProjectError),
    #[cfg(feature = "js")]
    #[error(transparent)]
    JsError(#[from] JavascriptError),
    #[error(transparent)]
    Infallible(#[from] Infallible)
}
