use crate::dependencies::project_dependency::{subproject_url, PROJECT_SCHEME};
use crate::identifier::{InvalidId, ProjectId};
use std::path::PathBuf;
use thiserror::Error;
use url::Url;

/// A resource location in assemble
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ResourceLocation {
    project: ProjectId,
    path: String,
    configuration: Option<String>,
}

impl ResourceLocation {
    pub fn new<I>(project: ProjectId, path: &str, configuration: I) -> Self
    where
        for<'a> I: Into<Option<&'a str>>,
    {
        Self {
            project,
            path: path.to_string(),
            configuration: configuration.into().map(|s| s.to_string()),
        }
    }

    pub fn project(&self) -> &ProjectId {
        &self.project
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn configuration(&self) -> Option<&str> {
        self.configuration.as_deref()
    }
}

impl From<ResourceLocation> for Url {
    fn from(r: ResourceLocation) -> Self {
        subproject_url(&r.project, &r.path, r.configuration).unwrap()
    }
}

impl TryFrom<Url> for ResourceLocation {
    type Error = InvalidResourceLocation;

    fn try_from(value: Url) -> Result<Self, Self::Error> {
        if value.scheme() != PROJECT_SCHEME {
            return Err(InvalidResourceLocation::BadSchema(
                value.scheme().to_string(),
            ));
        }

        let path = value.path();
        if path.ends_with("/") {
            // use default configuration
            let path = PathBuf::from(path);
            let id = ProjectId::from_path(path)?;

            Ok(Self {
                project: id,
                path: "".to_string(),
                configuration: None,
            })
        } else {
            // last element is configuration
            let path = PathBuf::from(path);
            let configuration = path.file_name().and_then(|os| os.to_str()).unwrap();
            let project = ProjectId::from_path(path.parent().unwrap())?;

            Ok(Self {
                project,
                path: "".to_string(),
                configuration: Some(configuration.to_string()),
            })
        }
    }
}

#[derive(Debug, Error)]
pub enum InvalidResourceLocation {
    #[error(
        "Unexpected schema found, must be {:?} (found = {0:?})",
        PROJECT_SCHEME
    )]
    BadSchema(String),
    #[error(transparent)]
    InvalidId(#[from] InvalidId),
}
