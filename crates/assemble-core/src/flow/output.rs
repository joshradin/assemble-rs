//! The outputs of the assemble project

use crate::dependencies::file_dependency::FILE_SYSTEM_TYPE;
use crate::dependencies::{
    AcquisitionError, Dependency, DependencyType, Registry, ResolvedDependency,
    ResolvedDependencyBuilder,
};
use crate::file_collection::FileSet;
use crate::flow::shared::{Artifact, ConfigurableArtifact, ImmutableArtifact, IntoArtifact};
use crate::properties::Provides;
use crate::task::{BuildableTask, ExecutableTask, ResolveInnerTask, TaskHandle};
use crate::{Executable, Task};
use std::collections::HashMap;
use std::convert::Infallible;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The artifact handler
#[derive(Default)]
pub struct ArtifactHandler {
    variant_map: HashMap<String, Arc<dyn Artifact>>,
}

impl ArtifactHandler {
    pub fn new() -> Self {
        Self {
            variant_map: Default::default(),
        }
    }

    /// Adds an artifact for a configuration
    pub fn add<S, A>(&mut self, variant: S, artifact: A) -> impl Artifact
    where
        S: AsRef<str>,
        A: IntoArtifact,
        A::IntoArtifact: 'static,
    {
        self.add_with(variant, artifact, |_| {})
    }

    pub fn add_with<S, A, F>(&mut self, configuration: S, artifact: A, config: F) -> impl Artifact
    where
        S: AsRef<str>,
        A: IntoArtifact,
        A::IntoArtifact: 'static,
        F: FnOnce(&mut ConfigurableArtifact),
    {
        let config_name = configuration.as_ref().to_string();
        let artifact = artifact.into_artifact();
        let mut configurable = ConfigurableArtifact::from_artifact(artifact);
        (config)(&mut configurable);
        self.variant_map
            .insert(config_name, Arc::new(configurable.clone()));
        configurable
    }

    pub(crate) fn get_artifact(&self, configuration: &str) -> Option<Arc<dyn Artifact>> {
        self.variant_map.get(configuration).map(|b| b.clone())
    }
}

pub trait SinglePathOutputTask : Task + Send + 'static {
    fn get_path(task: &Executable<Self>) -> PathBuf;
}

impl <T : SinglePathOutputTask> ArtifactTask for T {
    fn get_artifact(task: &Executable<Self>) -> ImmutableArtifact {
        ImmutableArtifact::new(T::get_path(task))
    }
}

impl <T : SinglePathOutputTask> Provides<PathBuf> for TaskHandle<T> {
    fn try_get(&self) -> Option<PathBuf> {
        self.provides(|e| T::get_path(e)).try_get()
    }
}

/// A task that produces an artifact
pub trait ArtifactTask: Task + Send + 'static {
    /// Get the artifact produced by this task.
    fn get_artifact(task: &Executable<Self>) -> ImmutableArtifact;
}

impl<AT: ArtifactTask> Dependency for Executable<AT> {
    fn id(&self) -> String {
        AT::get_artifact(self).file().to_str().unwrap().to_string()
    }

    fn dep_type(&self) -> DependencyType {
        FILE_SYSTEM_TYPE.clone()
    }

    fn try_resolve(
        &self,
        _: &dyn Registry,
        _: &Path,
    ) -> Result<ResolvedDependency, AcquisitionError> {
        Ok(ResolvedDependencyBuilder::new(AT::get_artifact(self))
            .built_by(self.built_by())
            .finish())
    }
}

impl<AT: ArtifactTask + Send + 'static> Dependency for TaskHandle<AT> {
    fn id(&self) -> String {
        self.provides(|s| s.id()).get()
    }

    fn dep_type(&self) -> DependencyType {
        FILE_SYSTEM_TYPE.clone()
    }

    fn try_resolve(
        &self,
        _: &dyn Registry,
        _: &Path,
    ) -> Result<ResolvedDependency, AcquisitionError> {
        Ok(
            ResolvedDependencyBuilder::new(self.provides(|s| AT::get_artifact(s)).get())
                .built_by(self.clone())
                .finish(),
        )
    }
}

impl<T: ArtifactTask> From<TaskHandle<T>> for FileSet {
    fn from(t: TaskHandle<T>) -> Self {
        let artifact = t.provides(|e| T::get_artifact(e)).get();
        let mut set = FileSet::from(artifact.file());
        set.built_by(t);
        set
    }
}
