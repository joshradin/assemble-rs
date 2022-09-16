//! The project error type. Should be returned during building of the project.

use std::sync::PoisonError;
use std::any::Any;
use std::io;
use std::convert::Infallible;
use std::fmt::Display;
use crate::__export::TaskId;
use crate::dependencies::AcquisitionError;
use crate::dependencies::project_dependency::ProjectUrlError;
use crate::error::PayloadError;
use crate::identifier::InvalidId;
use crate::plugins::extensions::ExtensionError;
use crate::plugins::PluginError;
use crate::lazy_evaluation;
use crate::lazy_evaluation::ProviderError;
use crate::resources::InvalidResourceLocation;
use crate::task::flags::{OptionsDecoderError, OptionsSlurperError};
use crate::workspace::WorkspaceError;

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("No task identifier could be found for {0:?}")]
    NoIdentifiersFound(String),
    #[error("Too many task identifiers found for {1}. Found {0:?}")]
    TooManyIdentifiersFound(Vec<TaskId>, String),
    #[error("Identifier Missing: {0}")]
    IdentifierMissing(TaskId),
    #[error(transparent)]
    InvalidIdentifier(#[from] InvalidId),
    #[error(transparent)]
    PluginError(#[from] PluginError),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error("Inner Error {{ ... }}")]
    SomeError {},
    #[error("Infallible error occurred")]
    Infallible(#[from] Infallible),
    #[error(transparent)]
    PropertyError(#[from] lazy_evaluation::Error),
    #[error(transparent)]
    WorkspaceError(#[from] WorkspaceError),
    #[error("Invalid Type for file: {0}")]
    InvalidFileType(String),
    #[error("RwLock poisoned")]
    PoisonError,
    #[error("Actions already queried")]
    ActionsAlreadyQueried,
    #[error("No shared project was set")]
    NoSharedProjectSet,
    #[error(transparent)]
    OptionsDecoderError(#[from] OptionsDecoderError),
    #[error(transparent)]
    OptionsSlurperError(#[from] OptionsSlurperError),
    #[error(transparent)]
    ProjectUrlError(#[from] ProjectUrlError),
    #[error(transparent)]
    InvalidResourceLocation(#[from] InvalidResourceLocation),
    #[error(transparent)]
    AcquisitionError(#[from] AcquisitionError),
    #[error("{0}")]
    CustomError(String),
    #[error(transparent)]
    ProviderError(#[from] ProviderError),
    #[error(transparent)]
    ExtensionError(#[from] ExtensionError),
}

impl<G> From<PoisonError<G>> for ProjectError {
    fn from(_: PoisonError<G>) -> Self {
        Self::PoisonError
    }
}

impl ProjectError {
    pub fn invalid_file_type<T>() -> Self {
        Self::InvalidFileType(std::any::type_name::<T>().to_string())
    }

    pub fn custom<E: Display + Send + Sync + 'static>(error: E) -> Self {
        Self::CustomError(error.to_string())
    }
}

impl From<Box<dyn Any + Send>> for ProjectError {
    fn from(e: Box<dyn Any + Send>) -> Self {
        Self::SomeError {}
    }
}

// impl<T> From<T> for PayloadError<ProjectError>
//     where T : Into<ProjectError> {
//     fn from(e: T) -> Self {
//         PayloadError::new(e.into())
//     }
// }


#[macro_export]
macro_rules! payload_from {
    ($ty:ty) => {
        impl<T> From<T> for $crate::error::PayloadError<$ty>
            where T : Into<$ty> {
            fn from(e: T) -> Self {
                $crate::error::PayloadError::new(e.into())
            }
        }

    };
}


payload_from!(ProjectError);



pub type Result<T> = std::result::Result<T, PayloadError<ProjectError>>;
pub type ProjectResult<T = ()> = Result<T>;
