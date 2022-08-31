//! Use github to get version info

use crate::exception::BuildException;
use crate::prelude::ProjectError;
use url::Url;

pub fn get_distribution_url(version_tag: &str) -> Result<Url, ProjectError> {
    let assets = get_assets_for_tag(version_tag).map_err(ProjectError::custom)?;

    todo!()
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
    use super::*;

    #[test]
    fn get_assets() {
        let assets = get_assets_for_tag("v0.0.0-prerelease1").unwrap();
        println!("assets = {:#?}", assets);
    }
}
