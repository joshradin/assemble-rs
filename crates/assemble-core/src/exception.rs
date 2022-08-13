//! Build time exceptions

use std::any::Any;
use std::error::Error;
use std::fmt::{Debug, Display};
use thiserror::Error;

#[derive(Debug)]
pub enum BuildException {
    StopAction,
    StopTask,
    Error(Box<dyn Any + Send + Sync>),
}

impl BuildException {
    pub fn new<E: 'static + Any + Send + Sync>(e: E) -> Self {
        let boxed: Box<dyn Any + Send + Sync> = Box::new(e);
        BuildException::Error(boxed)
    }

    pub fn custom(e: &str) -> Self {
        let boxed: Box<dyn Any + Send + Sync> = Box::new(e.to_string());
        BuildException::Error(boxed)
    }
}

impl<E: 'static + Error + Send + Sync> From<E> for BuildException {
    fn from(e: E) -> Self {
        Self::new(e)
    }
}

pub type BuildResult<T = ()> = Result<T, BuildException>;

/// Represents any error
#[derive(Debug, thiserror::Error)]
#[error("{}", message)]
pub struct BuildError {
    message: String,
}

impl BuildError {
    /// Create a new, arbitrary build error
    pub fn new(message: impl AsRef<str>) -> Self {
        Self {
            message: message.as_ref().to_string(),
        }
    }
}
