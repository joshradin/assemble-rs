use crate::dependencies::project_dependency::{subproject_url, ProjectUrlError, PROJECT_SCHEME};
use crate::flow::shared::Artifact;
use crate::identifier::{InvalidId, ProjectId};
use crate::project::{GetProjectId, VisitProject};
use crate::Project;
use crate::__export::TaskId;
use crate::lazy_evaluation::Provider;
use crate::prelude::ProjectResult;
use crate::project::buildable::Buildable;

use std::collections::HashSet;
use std::path::PathBuf;
use thiserror::Error;
use url::Url;

/// A resource location in assemble
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ResourceLocation {
    project: ProjectId,
    configuration: Option<String>,
}

impl ResourceLocation {
    pub fn new<'a, I>(project: ProjectId, configuration: I) -> Self
    where
        I: Into<Option<&'a str>>,
    {
        Self {
            project,
            configuration: configuration.into().map(|s| s.to_string()),
        }
    }

    pub fn find<'a, P, I>(
        project: &P,
        path: &str,
        configuration: I,
    ) -> Result<Self, InvalidResourceLocation>
    where
        P: GetProjectId,
        I: Into<Option<&'a str>>,
    {
        let url = subproject_url(project, path, configuration.into().map(str::to_string))?;
        Self::try_from(url)
    }

    pub fn project(&self) -> &ProjectId {
        &self.project
    }

    pub fn configuration(&self) -> Option<&str> {
        self.configuration.as_deref()
    }
}

impl From<ResourceLocation> for Url {
    fn from(r: ResourceLocation) -> Self {
        subproject_url(&r.project, "", r.configuration).unwrap()
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
        if path.ends_with('/') {
            // use default configuration
            let path = PathBuf::from(path);
            let id = ProjectId::try_from(path.as_path())?;

            Ok(Self::new(id, None))
        } else {
            // last element is configuration
            let path = PathBuf::from(path);
            let configuration = path.file_name().and_then(|os| os.to_str()).unwrap();
            let project = ProjectId::try_from(path.parent().unwrap())?;

            Ok(Self::new(project, configuration))
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
    #[error(transparent)]
    ProjectUrlError(#[from] ProjectUrlError),
    #[error("No resources could be found")]
    NoResourceFound,
}

/// A project visitor that tries to find a resource
pub struct ResourceLocator {
    location: ResourceLocation,
}

impl ResourceLocator {
    pub fn new(location: ResourceLocation) -> Self {
        Self { location }
    }
}

impl VisitProject<Option<Box<dyn Artifact>>> for ResourceLocator {
    fn visit(&mut self, project: &Project) -> Option<Box<dyn Artifact>> {
        let mut project_ptr = project.root_project();

        for part in self.location.project.iter().skip(1) {
            project_ptr = project_ptr.with(|p| p.get_subproject(part).ok().cloned())?;
        }

        let artifact = project_ptr.with(|p| {
            let configuration = self
                .location
                .configuration
                .as_ref()
                .cloned()
                .unwrap_or(p.variants().default());

            p.variant(&configuration)
        })?;
        Some(Box::new(artifact.get()))
    }
}

pub trait ProjectResourceExt {
    /// Try to get a resource from a project.
    fn get_resource<R>(&self, resource: R) -> Result<Box<dyn Artifact>, InvalidResourceLocation>
    where
        R: TryInto<ResourceLocation>;
}

impl ProjectResourceExt for Project {
    fn get_resource<R>(&self, resource: R) -> Result<Box<dyn Artifact>, InvalidResourceLocation>
    where
        R: TryInto<ResourceLocation>,
    {
        let location = resource
            .try_into()
            .map_err(|_| InvalidResourceLocation::NoResourceFound)?;
        let mut visitor = ResourceLocator::new(location);
        self.visitor(&mut visitor)
            .ok_or(InvalidResourceLocation::NoResourceFound)
    }
}

impl Buildable for ResourceLocation {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        let resource = project.get_resource(self.clone())?;
        match resource.buildable() {
            None => Ok(HashSet::new()),
            Some(b) => b.get_dependencies(project),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::ProjectId;
    use crate::resources::ResourceLocation;
    use std::str::FromStr;

    #[test]
    fn url_conversion() {
        let resource1 = ResourceLocation::new(ProjectId::from_str(":root").unwrap(), None);
        let as_url = Url::from(resource1.clone());
        assert_eq!(ResourceLocation::try_from(as_url).unwrap(), resource1);

        let resource2 = (ResourceLocation::find(
            &ProjectId::from_str(":root").unwrap(),
            "child1:child2",
            Some("jar"),
        ))
        .unwrap();
        assert_eq!(
            resource2.project,
            ProjectId::from_str(":root:child1:child2").unwrap()
        );
        let as_url = Url::from(resource2.clone());
        assert_eq!(ResourceLocation::try_from(as_url).unwrap(), resource2);
    }
}
