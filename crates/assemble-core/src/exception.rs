//! Build time exceptions

use crate::error::PayloadError;
use crate::project::buildable::Buildable;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use thiserror::Error;

pub enum BuildException {
    StopAction,
    StopTask,
    Error(Box<dyn Display + Send + Sync>),
}

impl BuildException {
    pub fn new<E: 'static + Display + Send + Sync>(e: E) -> Self {
        let boxed: Box<dyn Display + Send + Sync> = Box::new(e);
        BuildException::Error(boxed)
    }

    pub fn custom(e: &str) -> Self {
        let boxed: Box<dyn Display + Send + Sync> = Box::new(e.to_string());
        BuildException::Error(boxed)
    }
}

impl<E: 'static + Error + Send + Sync> From<E> for BuildException {
    fn from(e: E) -> Self {
        Self::new(e)
    }
}

impl Debug for BuildException {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildException::StopAction => f.debug_struct("StopAction").finish(),
            BuildException::StopTask => f.debug_struct("StopTask").finish(),
            BuildException::Error(e) => f
                .debug_struct("Error")
                .field("inner", &e.to_string())
                .finish(),
        }
    }
}

impl Display for BuildException {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildException::StopAction => f.debug_struct("StopAction").finish(),
            BuildException::StopTask => f.debug_struct("StopTask").finish(),
            BuildException::Error(e) => write!(f, "{}", e),
        }
    }
}

impl<T> From<T> for PayloadError<BuildException>
where
    T: Into<BuildException>,
{
    fn from(err: T) -> Self {
        PayloadError::new(err.into())
    }
}

pub type BuildResult<T = ()> = Result<T, PayloadError<BuildException>>;

/// Represents any error
#[derive(Debug)]
pub struct BuildError {
    message: String,
    inner: Option<Box<dyn Error + Send + Sync>>,
}

impl BuildError {
    /// Create a new, arbitrary build error
    pub fn new(message: impl AsRef<str>) -> Self {
        Self {
            message: message.as_ref().to_string(),
            inner: None,
        }
    }

    /// Create a new, arbitrary build error
    pub fn with_inner<S: AsRef<str>, E: Error + Send + Sync + 'static>(
        message: S,
        error: E,
    ) -> Self {
        Self {
            message: message.as_ref().to_string(),
            inner: Some(Box::new(error)),
        }
    }
}


impl Display for BuildError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            None => {
                write!(f, "{}", self.message)
            }
            Some(e) => {
                write!(f, "{} (inner = {})", self.message, e)
            }
        }
    }
}

impl Error for BuildError {}
