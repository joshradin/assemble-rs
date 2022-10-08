use crate::dependencies::{
    AcquisitionError, Dependency, DependencyType, Registry, ResolvedDependency,
    ResolvedDependencyBuilder,
};
use crate::file_collection::{FileCollection, FileSet};
use crate::project::buildable::{BuildableObject, GetBuildable};

use itertools::Itertools;
use once_cell::sync::Lazy;
use std::collections::VecDeque;

use std::path::{Path, PathBuf};
use url::Url;

#[derive(Debug)]
pub struct FileSystem(PathBuf);

#[cfg(windows)]
impl Default for FileSystem {
    fn default() -> Self {
        FileSystem(PathBuf::from(r"c:\"))
    }
}

#[cfg(unix)]
impl Default for FileSystem {
    fn default() -> Self {
        FileSystem(PathBuf::from(r"/"))
    }
}

impl Registry for FileSystem {
    fn url(&self) -> Url {
        Url::from_directory_path(self.0.clone())
            .unwrap_or_else(|_| panic!("couldn't treat {:?} as root directory for URL", self.0))
    }

    fn supported(&self) -> Vec<DependencyType> {
        vec![FILE_SYSTEM_TYPE.clone()]
    }
}

/// The file system dependency type. Just represents a normal
pub static FILE_SYSTEM_TYPE: Lazy<DependencyType> =
    Lazy::new(|| DependencyType::new("file", "direct_file_url", ["*"]));

impl GetBuildable for PathBuf {
    fn as_buildable(&self) -> BuildableObject {
        BuildableObject::None
    }
}

impl Dependency for PathBuf {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn dep_type(&self) -> DependencyType {
        FILE_SYSTEM_TYPE.clone()
    }

    fn try_resolve(
        &self,
        registry: &dyn Registry,
        _cache_path: &Path,
    ) -> Result<ResolvedDependency, AcquisitionError> {
        let url = registry.url();
        if url.scheme() != "file" {
            return Err(AcquisitionError::IncorrectUrlScheme {
                found: url.scheme().to_string(),
                expected: "file".to_string(),
            });
        }
        println!("registry url = {}", url);
        let file_system_url = url
            .to_file_path()
            .unwrap_or_else(|_| panic!("couldn't treat {} as a path", url));
        if self.is_absolute() {
            Ok(ResolvedDependencyBuilder::new(self.clone()).finish())
        } else {
            let path = file_system_url.join(self);
            Ok(ResolvedDependencyBuilder::new(path).finish())
        }
    }
}

impl Dependency for FileSet {
    fn id(&self) -> String {
        format!("{:?}", self.path().unwrap())
    }

    fn dep_type(&self) -> DependencyType {
        FILE_SYSTEM_TYPE.clone()
    }

    fn try_resolve(
        &self,
        registry: &dyn Registry,
        cache_path: &Path,
    ) -> Result<ResolvedDependency, AcquisitionError> {
        let mut paths = self.files().into_iter().collect::<VecDeque<_>>();
        let first = paths.pop_front().ok_or(AcquisitionError::MissingFile)?;
        let output = first.try_resolve(registry, Path::new(""))?;
        paths
            .into_iter()
            .map(|path| path.try_resolve(registry, cache_path))
            .fold(
                Ok(output),
                |accum, next| -> Result<ResolvedDependency, AcquisitionError> {
                    let accum = accum?;
                    let next = next?;
                    Ok(accum.join(next))
                },
            )
    }
}
