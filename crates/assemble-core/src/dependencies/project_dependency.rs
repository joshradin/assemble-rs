//! Provides the project dependency trait for dependency containers

use crate::dependencies::{
    AcquisitionError, Dependency, DependencyType, Registry, ResolvedDependency,
    ResolvedDependencyBuilder,
};
use crate::flow::shared::Artifact;
use crate::identifier::{Id, InvalidId};
use crate::plugins::Plugin;
use crate::prelude::{ProjectId, SharedProject};
use crate::project::buildable::{Buildable, BuildableObject, GetBuildable};
use crate::project::error::ProjectResult;
use crate::project::GetProjectId;
use crate::resources::{ProjectResourceExt, ResourceLocation};
use crate::Project;
use crate::__export::TaskId;

use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;
use std::fmt::Debug;
use std::path::Path;

use url::Url;

/// Get access to project dependencies
pub trait CreateProjectDependencies {
    /// Creates an inter-project dependency with the default configuration
    fn project<S: AsRef<str>>(&self, path: S) -> ProjectDependency;
    /// Creates an inter-project dependency with a given configuration
    fn project_with<P: AsRef<str>, C: AsRef<str>>(&self, path: P, config: C) -> ProjectDependency;
}

impl CreateProjectDependencies for Project {
    fn project<S: AsRef<str>>(&self, path: S) -> ProjectDependency {
        ProjectDependency {
            parent: self.as_shared(),
            location: ResourceLocation::find(self.id(), path.as_ref(), None)
                .expect("no project found"),
        }
    }

    fn project_with<P: AsRef<str>, C: AsRef<str>>(&self, path: P, config: C) -> ProjectDependency {
        ProjectDependency {
            parent: self.as_shared(),
            location: ResourceLocation::find(self.id(), path.as_ref(), config.as_ref())
                .expect("no project found"),
        }
    }
}

impl CreateProjectDependencies for SharedProject {
    fn project<S: AsRef<str>>(&self, path: S) -> ProjectDependency {
        ProjectDependency {
            parent: self.clone(),
            location: ResourceLocation::find(&self.project_id(), path.as_ref(), None)
                .expect("no project found"),
        }
    }

    fn project_with<P: AsRef<str>, C: AsRef<str>>(&self, path: P, config: C) -> ProjectDependency {
        ProjectDependency {
            parent: self.clone(),
            location: ResourceLocation::find(&self.project_id(), path.as_ref(), config.as_ref())
                .expect("no project found"),
        }
    }
}

#[derive(Debug)]
pub struct ProjectDependency {
    parent: SharedProject,
    location: ResourceLocation,
}

impl Buildable for ProjectDependency {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        let location = self.location.clone();
        self.parent.with(|p| {
            let resource = p
                .get_resource(location)
                .map_err(|e| AcquisitionError::custom(e.to_string()))
                .unwrap();

            match resource.buildable() {
                None => Ok(HashSet::new()),
                Some(buildable) => buildable.get_dependencies(project),
            }
        })
    }
}

impl GetBuildable for ProjectDependency {
    fn as_buildable(&self) -> BuildableObject {
        let location = self.location.clone();
        self.parent.with(|p| {
            let resource = p
                .get_resource(location)
                .map_err(|e| AcquisitionError::custom(e.to_string()))
                .unwrap();

            match resource.buildable() {
                None => BuildableObject::None,
                Some(buildable) => BuildableObject::from(buildable),
            }
        })
    }
}

impl Dependency for ProjectDependency {
    fn id(&self) -> String {
        format!("{}", self.location.project())
    }

    fn dep_type(&self) -> DependencyType {
        PROJECT_DEPENDENCY_TYPE.clone()
    }

    fn try_resolve(
        &self,
        _: &dyn Registry,
        _: &Path,
    ) -> Result<ResolvedDependency, AcquisitionError> {
        let location = self.location.clone();
        self.parent.with(|p| {
            let resource = p
                .get_resource(location)
                .map_err(|e| AcquisitionError::custom(e.to_string()))?;

            Ok(ResolvedDependencyBuilder::new(resource).finish())
        })
    }

    // fn maybe_buildable(&self) -> Option<Box<dyn Buildable>> {
    //     let location = self.location.clone();
    //     self.parent.with(|p| {
    //         let resource = p
    //             .get_resource(location)
    //             .map_err(|e| AcquisitionError::custom(e.to_string()))
    //             .unwrap();
    //
    //         resource.buildable()
    //     })
    // }
}

/// The dependency type of project outgoing variants
pub static PROJECT_DEPENDENCY_TYPE: Lazy<DependencyType> =
    Lazy::new(|| DependencyType::new("project", "project_variant_artifact", vec!["*"]));

/// Allows using projects to resolve project dependencies
pub struct ProjectRegistry;

impl ProjectRegistry {
    fn new() -> Self {
        Self
    }
}

impl Registry for ProjectRegistry {
    fn url(&self) -> Url {
        Url::parse("https://localhost:80/").unwrap()
    }

    fn supported(&self) -> Vec<DependencyType> {
        vec![PROJECT_DEPENDENCY_TYPE.clone()]
    }
}

#[derive(Debug, Default)]
pub struct ProjectDependencyPlugin;

impl Plugin for ProjectDependencyPlugin {
    fn apply(&self, project: &mut Project) -> ProjectResult {
        for sub in project.subprojects() {
            sub.apply_plugin::<Self>()?;
        }
        project.registries_mut(|reg| {
            reg.add_registry(ProjectRegistry::new());
            Ok(())
        })?;
        Ok(())
    }
}

pub static PROJECT_SCHEME: &str = "assemble";

pub fn project_url<P: GetProjectId>(project: &P) -> Url {
    let id = project.project_id();
    _project_url(id)
}

fn _project_url(id: ProjectId) -> Url {
    let project_as_path = id.iter().join("/");
    let host = "project.assemble.rs";
    Url::parse(&format!(
        "{scheme}://{host}/{path}/",
        scheme = PROJECT_SCHEME,
        path = project_as_path
    ))
    .unwrap()
}

pub fn subproject_url<P: GetProjectId>(
    base_project: &P,
    path: &str,
    configuration: impl Into<Option<String>>,
) -> Result<Url, ProjectUrlError> {
    static PROJECT_LOCATOR_REGEX: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(:{0,2})([a-zA-Z]\w*)").unwrap());

    let project_ptr = if path.is_empty() {
        base_project.project_id()
    } else {
        let mut project_ptr: Option<Id> = None;

        for captures in PROJECT_LOCATOR_REGEX.captures_iter(path) {
            let mechanism = &captures[1];
            let id = &captures[2];

            match mechanism {
                ":" => match project_ptr {
                    None => {
                        project_ptr = Some(Id::new(id)?);
                    }
                    Some(s) => {
                        project_ptr = Some(s.join(id)?);
                    }
                },
                "::" => match project_ptr {
                    None => {
                        project_ptr = Some(
                            base_project
                                .parent_id()
                                .ok_or(InvalidId::new("No parent id"))
                                .and_then(|parent| parent.join(id))?,
                        )
                    }
                    Some(s) => {
                        project_ptr = Some(
                            s.parent()
                                .ok_or(InvalidId::new("No parent id"))
                                .and_then(|parent| parent.join(id))?,
                        )
                    }
                },
                "" => {
                    match project_ptr {
                        None => project_ptr = Some(base_project.project_id().join(id)?),
                        Some(_) => {
                            panic!("Shouldn't be possible to access a non :: or : access after the first")
                        }
                    }
                }
                s => {
                    panic!("{:?} should not be matchable", s)
                }
            }
        }
        project_ptr.unwrap().into()
    };

    let output: Url = project_url(&project_ptr);

    if let Some(configuration) = configuration.into() {
        Ok(output.join(&configuration)?)
    } else {
        Ok(output)
    }
}

impl GetProjectId for Url {
    fn project_id(&self) -> ProjectId {
        if self.scheme() != PROJECT_SCHEME {
            panic!("only assemble: scheme supported for project id urls")
        }
        ProjectId::from_path(self.path()).expect("url is not valid project id")
    }

    fn parent_id(&self) -> Option<ProjectId> {
        self.project_id().parent().cloned().map(ProjectId::from)
    }

    fn root_id(&self) -> ProjectId {
        let id = self.project_id();
        id.ancestors().last().cloned().map(ProjectId::from).unwrap()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectUrlError {
    #[error("No parent project to resolve non-absolute project path (path = {0:?})")]
    NoParentProject(String),
    #[error(transparent)]
    ParseUrlError(#[from] url::ParseError),
    #[error(transparent)]
    InvalidId(#[from] InvalidId),
    #[error("No project was found")]
    NoProjectFound,
}

impl GetProjectId for ProjectId {
    fn project_id(&self) -> ProjectId {
        self.clone()
    }

    fn parent_id(&self) -> Option<ProjectId> {
        self.parent().cloned().map(ProjectId::from)
    }

    fn root_id(&self) -> ProjectId {
        self.ancestors()
            .last()
            .cloned()
            .map(ProjectId::from)
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn create_project_url() {
        let project = Project::temp("root");
        let url = project_url(&project);
        println!("url = {}", url);

        assert_eq!(url.scheme(), PROJECT_SCHEME);
        assert_eq!(url.path(), "/root/");
    }

    #[test]
    fn url_as_assemble_project() {
        let child1 = ProjectId::from_str(":root:child1").unwrap();
        let _child2 = ProjectId::from_str(":root:child2").unwrap();

        let url = subproject_url(&child1, ":root:child1::child2", None).unwrap();

        println!("url = {}", url);
    }
}
