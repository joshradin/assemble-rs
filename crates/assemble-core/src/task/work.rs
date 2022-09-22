//! Traits and implementations for simplifying work model

use crate::project::ProjectResult;
use crate::task::work_handler::WorkHandler;

/// Add some type to a work handle.
///
/// When using `TaskIO` or `AddWork` derive, accessed using `#[flatten]` for inner
/// fields.
pub trait AddWork {
    /// Adds this object to a work handler, registering inputs and outputs
    /// as applicable.
    fn add_work(&self, handle: &mut WorkHandler) -> ProjectResult;
}

/// Add some input to a work handler
pub trait AddWorkInput {
    /// Adds this object to a work handler, registering inputs only
    fn add_input(&self, handle: &mut WorkHandler) -> ProjectResult;
}

/// Add some output to a work handler
pub trait AddWorkOutput {
    /// Adds this object to a work handler, registering output only
    fn add_output(&self, handle: &mut WorkHandler) -> ProjectResult;
}
