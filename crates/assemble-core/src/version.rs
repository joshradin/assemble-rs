//! Provide version information about assemble, generated at compile time.

use std::fmt::{Display, Formatter};

/// Version information about this version of assemble
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Version {
    name: String,
    version: String,
}

impl Version {
    /// Get an instance of the version
    fn instance() -> Self {
        let name = env!("CARGO_PKG_NAME");
        let version = env!("CARGO_PKG_VERSION");
        Self {
            name: name.to_string(),
            version: version.to_string(),
        }
    }

    /// Get the name of the package. Should always return `assemble-core`
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the version of the package.
    pub fn version(&self) -> &str {
        &self.version
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.version)
    }
}

/// Get version information about assemble
pub fn version() -> Version {
    Version::instance()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_version_info() {
        let version = version();
        assert_eq!(version.name(), "assemble-core");
        let semver = semver::Version::parse(version.version()).unwrap();
        let added = semver::Version::new(0, 1, 2);
        assert!(semver >= added);
    }
}
