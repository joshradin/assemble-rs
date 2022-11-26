//! Error result

use crate::builders::BuildConfigurator;
use assemble_freight::utils::FreightError;

#[derive(Debug, thiserror::Error)]
pub enum AssembleError {
    #[error(transparent)]
    FreightError(#[from] FreightError),
}
