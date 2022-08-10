//! Provides the project dependency trait for dependency containers

use crate::dependencies::file_dependency::FILE_SYSTEM_TYPE;
use crate::dependencies::{
    AcquisitionError, Dependency, DependencyType, Registry, ResolvedDependency,
};
use crate::identifier::{Id, InvalidId};
use crate::plugins::Plugin;
use crate::prelude::{ProjectId, SharedProject};
use crate::project::{GetProjectId, ProjectResult};
use crate::Project;
use itertools::Itertools;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::{Path, PathBuf};
use url::{ParseOptions, Url};

pub trait DependencyContainerProjectExt {
    /// Creates a inter-project dependency
    fn project<S: AsRef<str>>(&self, path: S) -> ProjectDependency;
}

pub struct ProjectDependency {
    root_project: SharedProject,
    project_path: String,
    configuration: String,
}

impl Dependency for ProjectDependency {
    fn id(&self) -> String {
        format!("{}:{}", self.project_path, self.configuration)
    }

    fn dep_type(&self) -> DependencyType {
        FILE_SYSTEM_TYPE.clone()
    }

    fn try_resolve(
        &self,
        _: &dyn Registry,
        _: &Path,
    ) -> Result<ResolvedDependency, AcquisitionError> {
        todo!()
    }
}

/// The dependency type of project outgoing variants
pub static PROJECT_DEPENDENCY_TYPE: Lazy<DependencyType> =
    Lazy::new(|| DependencyType::new("project", "project_variant_artifact", vec!["*"]));

/// Allows using projects to resolve project dependencies
pub struct ProjectRegistry {
    base_project: SharedProject,
}

impl ProjectRegistry {
    fn new(base_project: SharedProject) -> Self {
        Self { base_project }
    }
}

impl Registry for ProjectRegistry {
    fn url(&self) -> Url {
        project_url(&self.base_project)
    }

    fn supported(&self) -> Vec<DependencyType> {
        vec![PROJECT_DEPENDENCY_TYPE.clone()]
    }
}

#[derive(Debug, Default)]
pub struct ProjectDependencyPlugin;

impl Plugin for ProjectDependencyPlugin {
    fn apply(&self, project: &mut Project) -> ProjectResult {
        let shared = project.as_shared();
        for sub in project.subprojects() {
            sub.apply_plugin::<Self>()?;
        }
        project.registries_mut(|reg| {
            reg.add_registry(ProjectRegistry::new(shared));
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
            "" => match project_ptr {
                None => project_ptr = Some(base_project.project_id().join(id)?),
                Some(_) => {
                    panic!("Shouldn't be possible to access a non :: or : access after the first")
                }
            },
            s => {
                panic!("{:?} should not be matchable", s)
            }
        }
    }

    let output: Url = project_url(&ProjectId::from(
        project_ptr.ok_or(ProjectUrlError::NoProjectFound)?.clone(),
    ));

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
        let child2 = ProjectId::from_str(":root:child2").unwrap();

        let url = subproject_url(&child1, ":root:child1::child2", None).unwrap();

        println!("url = {}", url);
    }
}
