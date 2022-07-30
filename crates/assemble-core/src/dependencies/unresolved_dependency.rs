use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
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
    /// Try to resolve a dependency in a registry. The `cache_path` is somewhere to write files into
    /// if necessary.
    fn try_resolve(&self, registry: &dyn Registry, cache_path: &Path) -> Result<ResolvedDependency, AcquisitionError>;

}

assert_obj_safe!(Dependency);


impl Debug for Box<dyn Dependency> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.id())
    }
}

impl Display for Box<dyn Dependency> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id())
    }
}

/// An error occurred while trying to download a dependency
#[derive(Debug, thiserror::Error)]
pub enum AcquisitionError {
    #[error("{}", error)]
    Custom { error: String },
    #[error("Can't acquire dependency because url is in wrong scheme (expected = {expected}, found = {found})")]
    IncorrectUrlScheme {
        found: String,
        expected: String
    },
    #[error("File is missing")]
    MissingFile,
    #[error("Errors: {}", inner.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(","))]
    InnerErrors { inner: Vec<AcquisitionError> }
}

impl AcquisitionError {
    /// Create a custom acquisition error
    pub fn custom(message: impl ToString) -> Self {
        Self::Custom { error: message.to_string() }
    }
}

impl FromIterator<AcquisitionError> for AcquisitionError {
    fn from_iter<T: IntoIterator<Item=AcquisitionError>>(iter: T) -> Self {
        Self::InnerErrors { inner: iter.into_iter().collect()}
    }
}
