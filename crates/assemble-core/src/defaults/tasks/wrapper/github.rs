//! Use github to get version info

use crate::prelude::ProjectError;
use once_cell::sync::Lazy;
use regex::Regex;
use strum::IntoEnumIterator;

use crate::defaults::tasks::wrapper::{Distribution, Os};
use crate::version::Version;
use url::Url;

/// Gets a list of all distribution urls for given version
pub fn get_distributions(version_tag: &str) -> Result<Vec<Distribution>, ProjectError> {
    static TAG_REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^v\d+\.\d+\.\d+$").expect("invalid regex"));
    if !TAG_REGEX.is_match(version_tag) {
        return Err(ProjectError::custom(format!(
            "{} is invalid version tag",
            version_tag
        )));
    }

    Os::iter()
        .map(|os: Os| -> Result<Distribution, ProjectError> {

            let mut url_string = format!("https://github.com/joshradin/assemble-rs/releases/download/{tag}/assemble-{os}-amd64", tag = version_tag);
            if Version::with_version(&version_tag.replace('v', "")).match_requirement(">0.1.2") {
                url_string = format!("{}-{}", url_string, version_tag);
            }

            let url = Url::parse(&url_string)
                .map_err(ProjectError::custom)
                ?;
            Ok(Distribution {
                url,
                os
            })
        })
        .collect::<Result<Vec<_>, ProjectError>>()
}

pub fn get_current_distributions() -> Result<Vec<Distribution>, ProjectError> {
    get_distributions(&format!("{}", crate::version::version()))
}

/// Get a distribution from a list of distribution
pub trait GetDistribution {
    /// Gets the relevant distribution for this result
    fn get_relevant(self) -> Option<Distribution>;
}

impl GetDistribution for Vec<Distribution> {
    fn get_relevant(self) -> Option<Distribution> {
        self.into_iter().find(|d| d.os == Os::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    #[cfg(target_os = "macos")]
    fn get_os() {
        let os = Os::default();
        let expected = Os::MacOs;

        assert_eq!(os, expected);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn get_os() {
        let os = Os::default();
        let expected = Os::Linux;

        assert_eq!(os, expected);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn get_os() {
        let os = Os::default();
        let expected = Os::Windows;
        assert_eq!(os, expected);
    }

    #[test]
    fn download_release() {
        let _tempdir = tempdir().expect("couldn't create temp directory");
        let version = "v0.1.2";

        let download_url = get_distributions(version)
            .expect("couldn't get version")
            .into_iter()
            .find(|d| d.os == Os::Linux)
            .expect("Couldn't get release");
        assert_eq!(download_url.url.to_string(), "https://github.com/joshradin/assemble-rs/releases/download/v0.1.2/assemble-linux-amd64");
    }

    #[test]
    fn newer_download_release() {
        let _tempdir = tempdir().expect("couldn't create temp directory");
        let version = "v0.1.3";

        let download_url = get_distributions(version)
            .expect("couldn't get version")
            .into_iter()
            .find(|d| d.os == Os::Linux)
            .expect("Couldn't get release");
        assert_eq!(download_url.url.to_string(), "https://github.com/joshradin/assemble-rs/releases/download/v0.1.3/assemble-linux-amd64-v0.1.3");
    }

    #[test]
    fn can_get_current_release() {
        let dists = get_current_distributions();
        assert!(matches!(dists, Ok(_)));
        let dists = dists.unwrap();
        println!("dists: {:?}", dists);
    }
}
