//! Handles the construction of the `:build-logic` project

use assemble_core::error;
use assemble_core::prelude::SharedProject;
use std::convert::Infallible;
use std::error::Error;

pub mod plugin;

/// A build logic object must be able to configure a blank project into a runnable state
pub trait BuildLogic {
    /// The error type of the build logic
    type Err: Error + Send + Sync + 'static;

    /// Configures the project
    fn configure(&mut self, project: &SharedProject) -> error::Result<(), Self::Err>;
}

#[derive(Default)]
pub struct NoOpBuildLogic;

impl BuildLogic for NoOpBuildLogic {
    type Err = Infallible;

    fn configure(&mut self, project: &SharedProject) -> error::Result<(), Self::Err> {
        Ok(())
    }
}
