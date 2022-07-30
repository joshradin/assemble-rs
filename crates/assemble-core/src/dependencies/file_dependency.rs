use std::collections::VecDeque;
use std::ops::Deref;
use std::path::PathBuf;
use itertools::Itertools;
use once_cell::sync::Lazy;
use url::Url;
use crate::dependencies::{AcquisitionError, Dependency, DependencyType, Registry, ResolvedDependency, ResolvedDependencyBuilder};
use crate::file_collection::FileCollection;

#[derive(Debug)]
pub struct FileSystem;

impl Registry for FileSystem {
    fn url(&self) -> Url {
        Url::from_directory_path("/").unwrap()
    }

    fn supported(&self) -> Vec<DependencyType> {
        vec![FILE_SYSTEM_TYPE.clone()]
    }
}

/// The file system dependency type. Just represents a normal
pub static FILE_SYSTEM_TYPE: Lazy<DependencyType> = Lazy::new(|| {
    DependencyType::new("file", "direct_file_url", ["*"])
});


impl Dependency for PathBuf {
    fn id(&self) -> String {
        format!("{:?}", self)
    }

    fn dep_type(&self) -> DependencyType {
        FILE_SYSTEM_TYPE.clone()
    }

    fn try_resolve(&self, registry: &dyn Registry) -> Result<ResolvedDependency, AcquisitionError> {
        let file_system_url = registry.url().to_file_path().map_err(|_| AcquisitionError::IncorrectUrlProtocol)?;
        if self.is_absolute() {
            Ok(ResolvedDependencyBuilder::new(self.as_path()).finish())
        } else {
            let path = file_system_url.join(self);
            Ok(ResolvedDependencyBuilder::new(path.as_path()).finish())
        }
    }
}

impl <F : FileCollection> Dependency for F {
    fn id(&self) -> String {
        format!("{:?}", self.path().unwrap())
    }

    fn dep_type(&self) -> DependencyType {
        FILE_SYSTEM_TYPE.clone()
    }

    fn try_resolve(&self, registry: &dyn Registry) -> Result<ResolvedDependency, AcquisitionError> {
        let mut paths = self.files().into_iter().collect::<VecDeque<_>>();
        let first = paths.pop_front().ok_or(AcquisitionError::MissingFile)?;
        let mut output = first.try_resolve(registry)?;
        paths.into_iter()
            .map(|path| path.try_resolve(registry))
            .fold(Ok(output), |accum, next| -> Result<ResolvedDependency, AcquisitionError> {
                let accum = accum?;
                let next = next?;
                Ok(accum.join(next))
            }
        )

    }
}