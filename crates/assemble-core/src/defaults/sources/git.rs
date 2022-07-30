//! Allows usage of urls as git servers and create dependencies of git repos

use std::path::{Path, PathBuf};
use url::Url;
use crate::dependencies::{AcquisitionError, Dependency, DependencyType, Registry, ResolvedDependency};

/// Utilize some git server as provider of dependencies.
pub struct GitServer {
    url: Url
}

impl GitServer {

    /// Create a server from a git url
    pub fn new(url: Url) -> Self {
        Self { url }
    }

    /// Create a git server registry using [`github.com`](https://www.github.com)
    pub fn github() -> Self {
        Self {
            url: Url::parse("https://www.github.com").unwrap()
        }
    }

}

/// Represents a dependency that can be downloaded by cloning a git repository.
pub struct GitRepo {
    username: String,
    repo: String,
    branch: Option<String>,
    build_repo: Box<dyn Fn(&Path) -> Result<PathBuf, String>>
}

impl Dependency for GitRepo {
    fn id(&self) -> String {
        format!("{}/{}", self.username, self.repo)
    }

    fn dep_type(&self) -> DependencyType {
        todo!()
    }

    fn try_resolve(&self, registry: &dyn Registry, cache_path: &Path) -> Result<ResolvedDependency, AcquisitionError> {
        todo!()
    }
}