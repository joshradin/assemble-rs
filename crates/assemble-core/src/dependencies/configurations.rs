//! A configuration has two states: resolved and unresolved

use crate::__export::TaskId;
use crate::dependencies::{
    AcquisitionError, Dependency, IntoDependency, RegistryContainer, ResolvedDependency,
};
use crate::file_collection::{FileCollection, FileSet};
use crate::flow::shared::{Artifact, ImmutableArtifact, IntoArtifact};
use crate::lazy_evaluation::Provider;
use crate::prelude::ProjectResult;
use crate::project::buildable::{Buildable, BuiltByContainer};
use crate::project::error::ProjectError;
use crate::Project;
use once_cell::sync::OnceCell;
use std::collections::HashSet;
use std::fmt::{write, Debug, Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct Configuration {
    inner: Arc<Mutex<ConfigurationInner>>,
}

impl Configuration {
    /// Create a new configuration
    pub(crate) fn new(name: &str, registry_container: &Arc<Mutex<RegistryContainer>>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(ConfigurationInner {
                name: name.to_string(),
                parents: vec![],
                dependencies: vec![],
                resolved: OnceCell::new(),
                registry_container: registry_container.clone(),
            })),
        }
    }

    fn inner<R, F: FnOnce(&mut ConfigurationInner) -> R>(&self, func: F) -> R {
        let mut inner = self.inner.lock().unwrap();
        (func)(&mut inner)
    }

    fn inner_mut<R, F: FnOnce(&mut ConfigurationInner) -> R>(&mut self, func: F) -> R {
        let mut inner = self.inner.lock().unwrap();
        if inner.resolved.get().is_some() {
            panic!("{} already resolved", inner);
        }
        (func)(&mut inner)
    }

    /// Gets the resolved form of this configuration.
    ///
    /// If the configuration hasn't been resolved yet, resolves it at this point.
    pub fn resolved(&self) -> Result<ResolvedConfiguration, AcquisitionError> {
        self.inner(ConfigurationInner::resolve)
    }

    /// Add a dependency to this configuration
    pub fn add_dependency<D: IntoDependency>(&mut self, dependency: D)
    where
        D::IntoDep: 'static + Send + Sync,
    {
        let dependency = dependency.into_dependency();
        self.inner_mut(move |config| config.dependencies.push(Box::new(dependency)))
    }

    /// Adds a configuration that this configuration extends from
    pub fn extends_from(&mut self, other: &Configuration) {
        self.inner_mut(|inner| {
            inner.parents.push(other.clone());
        })
    }
}

impl Display for Configuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Configuration {:?}", self.inner.lock().unwrap().name)
    }
}

impl Provider<FileSet> for Configuration {
    fn try_get(&self) -> Option<FileSet> {
        self.resolved().ok().map(|config| {
            let files = config.files();
            let mut set = FileSet::from_iter(files);
            set.built_by(config);
            set
        })
    }
}

impl Buildable for Configuration {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        self.inner.lock()?.get_dependencies(project)
    }
}

struct ConfigurationInner {
    name: String,
    parents: Vec<Configuration>,
    dependencies: Vec<Box<dyn Dependency + Send + Sync>>,
    resolved: OnceCell<ResolvedConfiguration>,

    registry_container: Arc<Mutex<RegistryContainer>>,
}

impl ConfigurationInner {
    fn resolve(&mut self) -> Result<ResolvedConfiguration, AcquisitionError> {
        let dependencies = self.dependencies.drain(..).collect::<Vec<_>>();

        self.resolved
            .get_or_try_init(|| {
                let mut resolved = vec![];

                'outer: for dependency in dependencies {
                    println!("attempting to resolve {}", dependency);
                    let registry_c = self.registry_container.lock().unwrap();
                    let mut errors = vec![];
                    let mut found = false;
                    for registry in registry_c.supported_registries(&dependency.dep_type()) {
                        match dependency.try_resolve(registry, &registry_c.cache_location()) {
                            Ok(resolved_dep) => {
                                resolved.push(resolved_dep);

                                found = true;
                                continue 'outer;
                            }
                            Err(e) => {
                                errors.push(e);
                            }
                        }
                    }

                    if !found {
                        return Err(AcquisitionError::from_iter(errors));
                    }
                }

                Ok(ResolvedConfiguration {
                    dependencies: resolved,
                })
            })
            .map(|res| res.clone())
    }
}

impl Buildable for ConfigurationInner {
    /// The dependencies to resolve this configuration
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        let mut output = HashSet::new();
        for dep in &self.dependencies {
            if let Some(buildable) = dep.maybe_buildable() {
                output.extend(buildable.get_dependencies(project)?);
            }
        }
        Ok(output)
    }
}

impl Debug for ConfigurationInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("{}", self))
            .field("parents", &self.parents)
            .field("dependencies", &self.dependencies.len())
            .field("is resolved", &self.resolved.get().is_some())
            .finish()
    }
}

impl Display for ConfigurationInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Configuration {:?}", self.name)
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedConfiguration {
    dependencies: Vec<ResolvedDependency>,
}

impl Display for ResolvedConfiguration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut list = f.debug_list();
        for dep in &self.dependencies {
            let artifacts = dep
                .artifacts()
                .into_iter()
                .map(|s| ImmutableArtifact::new(s).file())
                .collect::<Vec<_>>();
            list.entries(artifacts);
        }
        list.finish()
    }
}

impl FileCollection for ResolvedConfiguration {
    fn files(&self) -> HashSet<PathBuf> {
        self.dependencies
            .iter()
            .flat_map(|dep| {
                let artifact_files = dep.artifact_files();
                artifact_files.files()
            })
            .collect()
    }
}

impl Buildable for ResolvedConfiguration {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        self.dependencies
            .iter()
            .map(|dep| dep.get_dependencies(project))
            .collect::<Result<Vec<_>, _>>()
            .map(|sets| sets.into_iter().flatten().collect())
    }
}
