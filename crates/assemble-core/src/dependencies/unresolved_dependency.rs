use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::io::Write;
use std::path::{Path, PathBuf};
use crate::dependencies::{DependencyType, Registry, ResolvedDependency};

/// An unresolved dependency. A dependency must be able to define what type dependency is and how
/// to download said repository.
pub trait Dependency {
    /// A way of identifying dependencies
    fn id(&self) -> String;
    /// The type of the dependency
    fn dep_type(&self) -> DependencyType;
    /// Try to resolve a dependency in a registry
    fn try_resolve(&self, registry: &dyn Registry) -> Result<ResolvedDependency, AcquisitionError>;

}

assert_obj_safe!(Dependency);

/// Define how to acquire a dependency
pub trait AcquireDependency{
    fn acquire(&self, values: HashMap<String, Box<dyn Any>>, registry: &dyn Registry, writer: &mut dyn Write) -> Result<(), AcquisitionError>;
}

assert_obj_safe!(AcquireDependency);

/// An error occurred while trying to download a dependency
#[derive(Debug, thiserror::Error)]
pub enum AcquisitionError {
    #[error("{}", error)]
    Custom { error: String },
    #[error("Can't acquire dependency because url is in wrong protocl")]
    IncorrectUrlProtocol,
    #[error("File is missing")]
    MissingFile
}

impl AcquisitionError {
    /// Create a custom acquisition error
    pub fn custom(message: impl ToString) -> Self {
        Self::Custom { error: message.to_string() }
    }
}

