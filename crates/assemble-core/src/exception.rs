//! Build time exceptions

use std::error::Error;
use std::fmt::{Debug, Display};
use thiserror::Error;

#[derive(Debug)]
pub enum BuildException {
    StopAction,
    StopTask,
    Error(Box<dyn Error + Send + Sync>),
}

impl BuildException {
    pub fn new<E: 'static + Error + Send + Sync>(e: E) -> Self {
        let boxed: Box<dyn Error + Send + Sync> = Box::new(e);
        BuildException::Error(boxed)
    }
}

impl<E: 'static + Error + Send + Sync> From<E> for BuildException {
    fn from(e: E) -> Self {
        Self::new(e)
    }
}

pub type BuildResult<T = ()> = Result<T, BuildException>;
