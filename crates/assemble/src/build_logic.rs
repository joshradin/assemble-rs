//! Handles the construction of the `:build-logic` project

use crate::error::AssembleError;
use assemble_core::error::PayloadError;
use assemble_core::prelude::SettingsAware;
use assemble_core::project::shared::SharedProject;
use std::convert::Infallible;
use std::error::Error;

pub mod plugin;

/// A build logic object must be able to configure a blank project into a runnable state
pub trait BuildLogic<S: SettingsAware> {
    /// The error type of the build logic
    type Err: Error + Send + Sync + 'static + Into<AssembleError>;

    /// Configures the project
    fn configure(
        &mut self,
        settings: &S,
        project: &SharedProject,
    ) -> Result<(), PayloadError<Self::Err>>;
}

#[derive(Default)]
pub struct NoOpBuildLogic;

impl<S: SettingsAware> BuildLogic<S> for NoOpBuildLogic {
    type Err = Infallible;

    fn configure(
        &mut self,
        _settings: &S,
        _project: &SharedProject,
    ) -> Result<(), PayloadError<Self::Err>> {
        Ok(())
    }
}
