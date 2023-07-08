//! Error result

use crate::builders::js::error::JavascriptError;
use crate::builders::BuildConfigurator;
use assemble_core::error::PayloadError;
use assemble_core::exception::BuildException;
use assemble_core::project::ProjectError;
use assemble_freight::utils::FreightError;
use std::convert::Infallible;

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
    Infallible(#[from] Infallible),
}
