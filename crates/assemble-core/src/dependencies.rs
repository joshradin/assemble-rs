//! Describes external dependencies

use reqwest::Url;

pub trait UnresolvedDependency: Sized {
    type Resolved: Dependency;

    fn try_resolve<'d, R: ?Sized>(
        self,
        resolver: &'d R,
    ) -> Result<Self::Resolved, (Self, DownloadError)>
    where
        R: DependencyResolver<'d>,
    {
        let key = self.create_key();
        let support = resolver.supporting_sources(&key);
        let mut errs = vec![];
        for source in support {
            match source.get_download_url(key.clone()) {
                Ok(download) => match self.download_dependency(download) {
                    Ok(resolved) => {
                        return Ok(resolved);
                    }
                    Err(e) => {
                        errs.push(e);
                    }
                },
                Err(e) => {
                    errs.push(e);
                }
            }
        }
        Err((self, DownloadError::Errors(errs)))
    }

    fn download_dependency(&self, url: Url) -> Result<Self::Resolved, DownloadError>;

    /// Create a dependency key
    fn create_key(&self) -> DependencyKey;
}

pub trait Dependency {
    fn id(&self) -> &str;
    fn source(&self) -> Url;
}

pub trait DependencyResolver<'a> {
    fn resolve_dependency<U>(&'a self, dependency: U) -> Result<U::Resolved, (U, DownloadError)>
    where
        U: UnresolvedDependency,
    {
        dependency.try_resolve(self)
    }

    fn sources(&self) -> Vec<&'a dyn Source>;

    fn supporting_sources(&'a self, key: &DependencyKey) -> Vec<&'a dyn Source> {
        self.sources()
            .into_iter()
            .filter(|source| source.supports_download(key))
            .collect()
    }
}

impl<'s, R: DependencyResolver<'s>> DependencyResolver<'s> for &R {
    fn sources(&self) -> Vec<&'s dyn Source> {
        (*self).sources()
    }
}

pub trait Source: Send + Sync {
    /// Check if this source can *support* this type of key. Doesn't check if key exists
    fn supports_download(&self, key: &DependencyKey) -> bool;
    fn get_download_url(&self, key: DependencyKey) -> Result<Url, DownloadError>;
}

assert_obj_safe!(Source);

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("{0:?}")]
    Errors(Vec<DownloadError>),
    #[error("Dependency not found")]
    NotFound,
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    ParseError(#[from] url::ParseError),
}

#[derive(Clone, Debug)]
pub enum DependencyKey {
    Crate {
        id: String,
        version: String,
    },
    Git {
        user: String,
        repo: String,
        branch: Option<String>,
    },
    Arbitrary {
        path: String,
    },
}

pub trait DependencyResolverFactory<'d, T: DependencyResolver<'d>> {
    fn get_resolver(&'d self) -> T;
}

pub trait ToDependency<T: UnresolvedDependency> {
    fn to_dep(self) -> T;
}

impl<U: UnresolvedDependency> ToDependency<Self> for U {
    fn to_dep(self) -> Self {
        self
    }
}
