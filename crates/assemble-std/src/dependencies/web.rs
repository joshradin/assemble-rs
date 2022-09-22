//! Simple web-based dependencies

use assemble_core::cryptography::hash_sha256;
use assemble_core::dependencies::{
    AcquisitionError, Dependency, DependencyType, Registry, ResolvedDependency,
    ResolvedDependencyBuilder,
};
use once_cell::sync::Lazy;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::{fs, io};
use std::fmt::{Debug, Formatter};
use url::Url;
use assemble_core::project::buildable::{BuildableObject, GetBuildable};

/// A web registry
pub struct WebRegistry {
    base_url: Url,
    name: String,
}

impl WebRegistry {
    /// Create a new web registry from a url
    pub fn new(name: &str, url: &str) -> Result<WebRegistry, url::ParseError> {
        Url::parse(url).map(|url| Self {
            base_url: url,
            name: name.to_string(),
        })
    }
}

impl Registry for WebRegistry {
    fn url(&self) -> Url {
        self.base_url.clone()
    }

    fn supported(&self) -> Vec<DependencyType> {
        vec![remote_file_system_type(&self.name)]
    }
}

/// Create a remote file system dependency type for a given name
pub fn remote_file_system_type(name: &str) -> DependencyType {
    DependencyType::new(name, name, ["*"])
}

/// A dependency that can be found on the web. This is an absolute path from a host
#[derive(Debug)]
pub struct WebDependency {
    file_path: PathBuf,
    from_registry: String,
    file_name: Option<OsString>,
}

impl WebDependency {
    /// Create a new web dependency
    pub fn new<P: AsRef<Path>, S: AsRef<str>>(file_path: P, from_registry: S) -> Self {
        Self {
            file_path: file_path.as_ref().to_path_buf(),
            from_registry: from_registry.as_ref().to_string(),
            file_name: None,
        }
    }

    /// Set an alternate file name to use.
    pub fn with_file_name(mut self, file: impl AsRef<OsStr>) -> Self {
        self.file_name = Some(file.as_ref().to_os_string());
        self
    }

    /// Gets the file name.
    pub fn file_name(&self) -> OsString {
        self.file_name
            .as_deref()
            .or(self.file_path.file_name())
            .unwrap_or(OsStr::new("tmp.bin"))
            .to_os_string()
    }
}

impl GetBuildable for WebDependency {
    fn as_buildable(&self) -> BuildableObject {
        BuildableObject::None
    }
}

impl Dependency for WebDependency {
    fn id(&self) -> String {
        self.file_name().to_string_lossy().to_string()
    }

    fn dep_type(&self) -> DependencyType {
        remote_file_system_type(&self.from_registry)
    }

    fn try_resolve(
        &self,
        registry: &dyn Registry,
        cache_path: &Path,
    ) -> Result<ResolvedDependency, AcquisitionError> {
        let registry_url = registry.url();
        let joined = registry_url
            .join(&self.file_path.to_string_lossy())
            .map_err(|e| AcquisitionError::custom(e))?;

        let file_name = self.file_name();

        let file_name_sha = hash_sha256(&format!("{}", joined));
        let download_location = cache_path
            .join("downloads")
            .join(file_name_sha.to_string())
            .join(file_name);

        fs::create_dir_all(download_location.parent().unwrap())
            .map_err(|e| AcquisitionError::custom(e))?;

        let response = reqwest::blocking::get(joined).map_err(|e| AcquisitionError::custom(e))?;

        let mut body = response.bytes().map_err(|e| AcquisitionError::custom(e))?;

        let mut file = File::options()
            .write(true)
            .create(true)
            .open(&download_location)
            .map_err(|e| AcquisitionError::custom(e))?;
        io::copy(&mut body.as_ref(), &mut file).map_err(|e| AcquisitionError::custom(e))?;

        Ok(ResolvedDependencyBuilder::new(download_location).finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assemble_core::file_collection::FileCollection;
    use std::env;
    use std::fs::create_dir_all;
    use tempfile::{tempdir, tempdir_in};

    #[test]
    fn download_rustup_init_script() {
        let registry = WebRegistry::new("rust-site", "https://sh.rustup.rs/").unwrap();
        let web_dependency = WebDependency::new("", "rust-site").with_file_name("rustup-init.sh");

        let current_dir = env::current_dir().unwrap();
        let temp_dir = tempdir_in(current_dir).expect("couldn't create temporary directory");

        let dependency = web_dependency
            .try_resolve(&registry, temp_dir.path())
            .unwrap();

        println!("dependency = {:#?}", dependency);
        assert_eq!(dependency.artifact_files().files().len(), 1)
    }
}
