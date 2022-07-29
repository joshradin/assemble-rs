//! Contains self resolving dependencies

use crate::dependencies::{Dependency, ModuleDependency};
use crate::file_collection::FileCollection;
use crate::flow::attributes::{AttributeContainer, ConfigurableAttributes, HasAttributes};
use crate::flow::shared::ConfigurableArtifact;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use std::sync::Arc;

pub use crate::prelude::*;
use crate::project::buildable::Buildable;
use crate::project::ProjectError;

/// A `SelfResolvingDependency` is a [`Dependency`](Dependency) which is able to resolve itself,
/// independent of a Registry
pub trait SelfResolvingDependency: Dependency + Buildable {
    /// Resolves this dependency using a specified transitive type
    fn resolve(&self, transitive: bool) -> HashSet<PathBuf>;
}

/// A dependency on a collection of local files
#[derive(Clone)]
pub struct FileCollectionDependency {
    collection: Arc<dyn FileCollection>,
    reason: String,
}

impl FileCollectionDependency {
    pub fn new<F: FileCollection + 'static>(file_c: F) -> Self {
        Self {
            collection: Arc::new(file_c),
            reason: "".to_string(),
        }
    }
}

impl Dependency for FileCollectionDependency {
    fn because<S: AsRef<str>>(&mut self, reason: S) {
        self.reason = reason.as_ref().to_string();
    }

    fn content_equals(&self, other: &Self) -> bool {
        self.collection.files() == other.collection.files()
    }

    fn identifier(&self) -> String {
        "file collection".to_string()
    }

    fn version(&self) -> Option<String> {
        None
    }

    fn reason(&self) -> &str {
        &self.reason
    }
}

impl Buildable for FileCollectionDependency {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        self.collection.get_dependencies(project)
    }
}

impl Debug for FileCollectionDependency {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.collection)
    }
}

impl SelfResolvingDependency for FileCollectionDependency {
    fn resolve(&self, _: bool) -> HashSet<PathBuf> {
        self.collection.files()
    }
}

#[derive(Debug, Clone)]
pub struct ProjectDependency {
    reason: String,
    project: SharedProject,
    attributes: AttributeContainer,
    requested_features: HashSet<String>,
    config: Option<String>,
}

impl ProjectDependency {
    pub const DEFAULT_CONFIGURATION: &'static str = "DEFAULT";
}

impl Dependency for ProjectDependency {
    fn because<S: AsRef<str>>(&mut self, reason: S) {
        self.reason = reason.as_ref().to_string();
    }

    fn content_equals(&self, other: &Self) -> bool {
        self.project.with(|p| p.id().clone()) == other.project.with(|p| p.id().clone())
    }

    fn identifier(&self) -> String {
        self.project.with(|p| p.id().clone()).to_string()
    }

    fn version(&self) -> Option<String> {
        None
    }

    fn reason(&self) -> &str {
        &self.reason
    }
}

impl Buildable for ProjectDependency {
    fn get_dependencies(&self, _: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        todo!()
    }
}

impl SelfResolvingDependency for ProjectDependency {
    fn resolve(&self, transitive: bool) -> HashSet<PathBuf> {
        todo!()
    }
}

impl ConfigurableAttributes for ProjectDependency {
    fn attributes<F: FnOnce(&mut AttributeContainer)>(&mut self, func: F) {
        (func)(&mut self.attributes)
    }
}

impl HasAttributes for ProjectDependency {
    fn get_attributes(&self) -> &AttributeContainer {
        &self.attributes
    }
}

impl ModuleDependency for ProjectDependency {
    fn artifacts(&self) -> HashSet<ConfigurableArtifact> {
        self.resolve(false)
            .into_iter()
            .map(|path| ConfigurableArtifact::from_artifact(path))
            .collect()
    }

    fn get_requested_features(&self) -> HashSet<String> {
        self.requested_features.clone()
    }

    fn target_configuration(&self) -> String {
        match self.config.as_ref() {
            None => ProjectDependency::DEFAULT_CONFIGURATION.to_string(),
            Some(s) => s.clone(),
        }
    }

    fn requested_features<I: IntoIterator<Item = String>>(&mut self, features: I) {
        self.requested_features = features.into_iter().collect();
    }

    fn set_target_configuration(&mut self, config: String) {
        self.config = Some(config);
    }
}
