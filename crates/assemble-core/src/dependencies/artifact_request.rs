//! Artifact request is a dependency that's requested using this following format:
//! `[group:]id:version`. Artifact requests can either be built manually, or actually parsed
//! from a `String` or `&str`.
//!
//! The `version` specifier should be parsable into a [semver version requirement](semver::VersionReq)

use crate::dependencies::{
    AcquisitionError, Dependency, DependencyType, IntoDependency, Registry, ResolvedDependency,
};
use crate::project::buildable::{BuildableObject, GetBuildable};

use once_cell::sync::Lazy;
use semver::VersionReq;

use std::path::Path;
use std::str::FromStr;
use thiserror::Error;

/// A request to get an artifact
#[derive(Debug)]
pub struct ArtifactRequest {
    group: Option<String>,
    module: String,
    config: String,
    semver_request: VersionReq,
}

impl FromStr for ArtifactRequest {
    type Err = ParseArtifactRequestError;

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

impl GetBuildable for ArtifactRequest {
    fn as_buildable(&self) -> BuildableObject {
        BuildableObject::None
    }
}

impl Dependency for ArtifactRequest {
    fn id(&self) -> String {
        todo!()
    }

    fn dep_type(&self) -> DependencyType {
        ARTIFACT_REQUEST_TYPE.clone()
    }

    fn try_resolve(
        &self,
        _registry: &dyn Registry,
        _cache_path: &Path,
    ) -> Result<ResolvedDependency, AcquisitionError> {
        todo!()
    }
}

/// The dependency type that accepts artifact requests
pub static ARTIFACT_REQUEST_TYPE: Lazy<DependencyType> = Lazy::new(|| {
    DependencyType::new(
        "artifact",
        "artifact_specified_dependency",
        ["*.*"], // only accept files
    )
});

/// An error occurred while trying to parse an artifact request
#[derive(Debug, Error)]
pub enum ParseArtifactRequestError {
    #[error(transparent)]
    SemverError(#[from] semver::Error),
}

impl IntoDependency for &str {
    type IntoDep = ArtifactRequest;

    fn into_dependency(self) -> Self::IntoDep {
        ArtifactRequest::from_str(self)
            .unwrap_or_else(|_| panic!("{self:?} is invalid artifact request"))
    }
}

macro_rules! string_into_dep {
    ($string:ty) => {
        impl IntoDependency for $string {
            type IntoDep = ArtifactRequest;

            fn into_dependency(self) -> Self::IntoDep {
                self.as_str().into_dependency()
            }
        }
    };
}
string_into_dep!(String);
string_into_dep!(&String);
