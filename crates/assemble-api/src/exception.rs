//! Build time exceptions

use std::error::Error;
use std::fmt::Debug;
use thiserror::Error;

pub trait DebugError: Error + Debug {}

#[derive(Debug, Error)]
pub enum BuildException {
    #[error("stop action requested")]
    StopAction,
    #[error("stop task requested")]
    StopTask,
    #[error(transparent)]
    Error(#[from] Box<dyn Error>),
}

impl BuildException {
    pub fn new<E: 'static + Error>(e: E) -> Self {
        let boxed: Box<dyn Error> = Box::new(e);
        BuildException::Error(boxed)
    }
}

pub type BuildResult<T = ()> = Result<T, BuildException>;
