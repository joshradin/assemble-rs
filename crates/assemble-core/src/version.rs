//! Provide version information about assemble, generated at compile time.

use semver::VersionReq;
use std::cmp::Ordering;
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

    /// Creates a new version instance with an arbitrary version and build
    pub fn new(build: &str, version: &str) -> Self {
        Self {
            name: build.to_string(),
            version: version.to_string(),
        }
    }

    /// Creates a new version instance with an arbitrary version, but for the `assemble-core` build
    pub fn with_version(version: &str) -> Self {
        Self::new(env!("CARGO_PKG_NAME"), version)
    }

    /// Get the name of the package. Should always return `assemble-core`
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the version of the package.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Check if this version matches some version requirement as expressed in Semver terms
    pub fn match_requirement(&self, req: &str) -> bool {
        let req = VersionReq::parse(req).expect(&format!("Invalid requirement string: {req:?}"));
        let semver = semver::Version::parse(&self.version)
            .expect(&format!("Invalid version string: {:?}", self.version));
        req.matches(&semver)
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

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.name != other.name {
            return None;
        }

        let this_version = semver::Version::parse(&self.version).ok()?;
        let other_version = semver::Version::parse(&other.version).ok()?;
        this_version.partial_cmp(&other_version)
    }
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
