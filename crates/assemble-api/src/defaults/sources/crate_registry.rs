use crate::dependencies::{
    Dependency, DependencyKey, DependencyResolver, DownloadError, Source, UnresolvedDependency,
};
use once_cell::sync::{Lazy, OnceCell};
use reqwest::{IntoUrl, Url};
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;

use thiserror::Error;
use url::UrlQuery;

pub type CrateName = String;

#[derive(Debug)]
pub struct CrateUnresolvedDependency {
    crate_name: CrateName,
    version: String,
    features: Vec<String>,
    use_default_features: bool,
}

impl CrateUnresolvedDependency {
    pub fn new(crate_name: CrateName, version: String) -> Self {
        Self {
            crate_name,
            version,
            features: vec![],
            use_default_features: true,
        }
    }
}

impl UnresolvedDependency for CrateUnresolvedDependency {
    type Resolved = CrateDependency;

    fn download_dependency(&self, url: Url) -> Result<Self::Resolved, DownloadError> {
        println!("Sending request to url: {}", url);
        let response = reqwest::blocking::get(url.clone())?;

        if response.status().is_success() {
            Ok(CrateDependency {
                id: self.crate_name.clone(),
                uri: url,
            })
        } else {
            Err(DownloadError::NotFound)
        }
    }

    fn create_key(&self) -> DependencyKey {
        DependencyKey::Crate {
            id: self.crate_name.clone(),
            version: self.version.clone(),
        }
    }
}

#[derive(Debug)]
pub struct CrateDependency {
    id: CrateName,
    uri: Url,
}

impl Dependency for CrateDependency {
    fn id(&self) -> &str {
        &self.id
    }

    fn source(&self) -> Url {
        self.uri.clone()
    }
}

#[derive(Debug, Deserialize)]
struct CrateIndex {
    dl: String,
    api: Option<String>,
}

pub struct CrateRegistry {
    index_url: Url,
    index: OnceCell<CrateIndex>,
}

impl Debug for CrateRegistry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "CrateRegistry {{ index_url = {} }}", self.index_url)
    }
}

impl CrateRegistry {
    pub fn new<I: IntoUrl>(url: I) -> Self {
        Self {
            index_url: url.into_url().expect("invalid url"),
            index: OnceCell::new(),
        }
    }

    pub fn crates_io() -> Self {
        Self::new("https://raw.githubusercontent.com/rust-lang/crates.io-index/master/config.json")
    }

    fn index(&self) -> Result<&CrateIndex, reqwest::Error> {
        self.index.get_or_try_init(|| {
            let url = &self.index_url;
            println!("attempting to get index at url: {}", url);
            reqwest::blocking::get(url.clone())?.json()
        })
    }

    fn format_strings(&self) -> &[&str] {
        &[
            "{crate}",
            "{version}",
            "{prefix}",
            "{lowerprefix}",
            "{sha256-checksum}",
        ]
    }
}

impl Source for CrateRegistry {
    fn supports_download(&self, key: &DependencyKey) -> bool {
        if let DependencyKey::Crate { .. } = key {
            true
        } else {
            false
        }
    }

    fn get_download_url(&self, key: DependencyKey) -> Result<Url, DownloadError> {
        if let DependencyKey::Crate { id, version } = key {
            let mut dl = self.index()?.dl.clone();
            if !dl.ends_with('/') {
                dl = format!("{dl}/");
            }
            let download_url = Url::parse(&dl)?;

            let crate_id = id;

            if self
                .format_strings()
                .iter()
                .any(|str| download_url.as_str().contains(str))
            {
                unimplemented!()
            } else {
                let url = download_url.join(&format!("{crate_id}/{version}/download"))?;
                Ok(url)
            }
        } else {
            unreachable!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_use_crates_io() {
        let crates_io = CrateRegistry::crates_io();

        let index = crates_io.index().unwrap();
        assert_eq!(index.dl, "https://crates.io/api/v1/crates");

        let dependency = CrateUnresolvedDependency::new("rand".to_string(), "0.8.5".to_string());

        let download_url = crates_io.get_download_url(dependency.create_key()).unwrap();
        println!("url: {}", download_url);
    }
}
