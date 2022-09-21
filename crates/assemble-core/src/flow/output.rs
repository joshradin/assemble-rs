//! The outputs of the assemble project

use crate::__export::{ProjectResult, TaskId};
use crate::dependencies::file_dependency::FILE_SYSTEM_TYPE;
use crate::dependencies::{
    AcquisitionError, Dependency, DependencyType, Registry, ResolvedDependency,
    ResolvedDependencyBuilder,
};
use crate::file_collection::FileSet;
use crate::flow::shared::{Artifact, ConfigurableArtifact, ImmutableArtifact, IntoArtifact};
use crate::identifier::Id;
use crate::lazy_evaluation::ProviderExt;
use crate::lazy_evaluation::{Prop, Provider};
use crate::project::buildable::{
    Buildable, BuildableObject, BuiltByContainer, GetBuildable, IntoBuildable,
};
use crate::task::{
    BuildableTask, ExecutableTask, HasTaskId, ResolveExecutable, ResolveInnerTask, TaskHandle,
};
use crate::{Executable, Project, Task};
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::env::var;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The outgoing variant handler
#[derive(Default)]
pub struct VariantHandler {
    default_variant: Option<String>,
    variant_map: HashMap<String, Prop<ConfigurableArtifact>>,
}

impl VariantHandler {
    pub fn new() -> Self {
        Self {
            default_variant: None,
            variant_map: Default::default(),
        }
    }

    /// Set the default outgoing variant
    pub fn set_default(&mut self, default: &str) {
        self.default_variant = Some(default.to_string());
    }

    /// Get the default variant name
    pub fn default(&self) -> String {
        self.default_variant
            .as_ref()
            .cloned()
            .or_else(|| {
                if self.variant_map.len() == 1 {
                    self.variant_map.keys().next().cloned()
                } else {
                    None
                }
            })
            .expect("no default variant could be determined")
    }

    /// Adds an artifact for a configuration
    pub fn add<S, A>(&mut self, variant: S, artifact: A)
    where
        S: AsRef<str>,
        A: IntoArtifact + Send + 'static,
        A::IntoArtifact: 'static,
    {
        let config_name = variant.as_ref().to_string();
        let mut prop = Prop::<ConfigurableArtifact>::new(Id::from(&*config_name));
        prop.set_with(Lazy::new(move || {
            let artifact = artifact.into_artifact();
            let buildable = artifact.buildable();
            let mut output = ConfigurableArtifact::from_artifact(artifact);
            if let Some(buildable) = buildable {
                output.built_by(buildable);
            }
            output
        }))
        .unwrap();

        self.variant_map.insert(config_name, prop);
    }

    pub fn add_with<S, A, F>(&mut self, configuration: S, artifact: A, config: F)
    where
        S: AsRef<str>,
        A: IntoArtifact + Send,
        A::IntoArtifact: 'static,
        F: FnOnce(&mut ConfigurableArtifact) + Send,
    {
        let config_name = configuration.as_ref().to_string();
        let mut prop = Prop::<ConfigurableArtifact>::new(Id::from(&*config_name));
        let artifact = artifact.into_artifact();
        let mut configurable = ConfigurableArtifact::from_artifact(artifact);
        (config)(&mut configurable);
        prop.set(configurable).unwrap();
        self.variant_map.insert(config_name, prop);
    }

    pub(crate) fn get_artifact(
        &self,
        configuration: &str,
    ) -> Option<impl Provider<ConfigurableArtifact>> {
        self.variant_map.get(configuration).map(|b| b.clone())
    }
}

pub trait SinglePathOutputTask: Task + Send + 'static {
    fn get_path(task: &Executable<Self>) -> PathBuf;
}

impl<T: SinglePathOutputTask> ArtifactTask for T {
    fn get_artifact(task: &Executable<Self>) -> ConfigurableArtifact {
        let mut output = ConfigurableArtifact::from_artifact(T::get_path(task));
        output.built_by(task.task_id().clone());
        output
    }
}

impl<T: SinglePathOutputTask> Provider<PathBuf> for TaskHandle<T> {
    fn try_get(&self) -> Option<PathBuf> {
        self.provides(|e| T::get_path(e)).try_get()
    }
}

/// A task that produces an artifact
pub trait ArtifactTask: Task + Send + 'static {
    /// Get the artifact produced by this task.
    fn get_artifact(task: &Executable<Self>) -> ConfigurableArtifact;
}

impl<A: ArtifactTask> IntoArtifact for TaskHandle<A> {
    type IntoArtifact = ConfigurableArtifact;

    fn into_artifact(self) -> Self::IntoArtifact {
        (&self).into_artifact()
    }
}

impl<A: ArtifactTask> IntoArtifact for &TaskHandle<A> {
    type IntoArtifact = ConfigurableArtifact;

    fn into_artifact(self) -> Self::IntoArtifact {
        self.provides(|s| A::get_artifact(s)).get()
    }
}

impl<AT: ArtifactTask> GetBuildable for Executable<AT> {
    fn as_buildable(&self) -> BuildableObject {
        BuildableObject::new(self.clone().into_buildable())
    }
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

    // fn maybe_buildable(&self) -> Option<Box<dyn Buildable>> {
    //     Some(Box::new(self.task_id().clone()))
    // }
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

    // fn maybe_buildable(&self) -> Option<Box<dyn Buildable>> {
    //     Some(Box::new(self.task_id().clone()))
    // }
}

impl<T: ArtifactTask> From<TaskHandle<T>> for FileSet {
    fn from(t: TaskHandle<T>) -> Self {
        let artifact = t.provides(|e| T::get_artifact(e)).get();
        let mut set = FileSet::from(artifact.file());
        set.built_by(t);
        set
    }
}
//
// /// Represents a variant that can be used as an output of a task
// pub struct Variant<A: ArtifactTask> {
//     outgoing_artifact: A,
//     built_by: BuiltByContainer,
// }
//
// impl<A: Artifact> Variant<A> {
//     /// Create a new variant using an outgoing variant
//     fn new<I>(outgoing_artifact: I) -> Self
//     where
//         I: IntoArtifact<IntoArtifact = A>,
//     {
//         let artifact = outgoing_artifact.into_artifact();
//         let built_by = artifact
//             .buildable()
//             .map(|b| BuiltByContainer::with_buildable(b))
//             .unwrap_or(BuiltByContainer::new());
//         Self {
//             outgoing_artifact: artifact,
//             built_by,
//         }
//     }
//
//     /// Add something that this variant is built by
//     pub fn built_by<T: IntoBuildable>(&mut self, buildable: T)
//     where
//         <T as IntoBuildable>::Buildable: 'static,
//     {
//         self.built_by.add(buildable)
//     }
// }
