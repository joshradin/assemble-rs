//! Use github to get version info

use crate::exception::BuildException;
use crate::prelude::ProjectError;
use url::Url;

pub fn get_distribution_url(version_tag: &str) -> Result<Url, ProjectError> {
    let assets = get_assets_for_tag(version_tag).map_err(ProjectError::custom)?;

    todo!()
}

pub fn get_current_distribution_url() -> Result<Url, ProjectError> {
    get_distribution_url(&format!("{}", crate::version::version()))
}

fn get_assets_for_tag(version_tag: &str) -> reqwest::Result<Vec<Asset>> {
    let response: ReleaseResponse = reqwest::blocking::get(format!(
        "https://api.github.com/repos/{owner}/{repo}/releases/tags/{tag}",
        owner = "joshRadin",
        repo = "assemble-rs",
        tag = version_tag
    ))?
    .error_for_status()?
    .json()?;
    Ok(response.assets)
}

#[derive(Debug, Deserialize)]
struct ReleaseResponse {
    name: String,
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    content_type: String,
    browser_download_url: Url,
}

#[cfg(test)]
mod tests {
    use tempfile::{TempDir, tempdir};
    use super::*;

    #[test]
    fn download_release() {
        let tempdir = tempdir().expect("couldn't create temp directory");
        let version = "0.1.2";

        let download_url = get_distribution_url(version).expect("couldn't get version");
        assert_eq!(download_url.to_string(), "https:/")
    }
}
