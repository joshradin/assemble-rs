//! A configuration has two states: resolved and unresolved

use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter, write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use once_cell::sync::OnceCell;
use crate::dependencies::{AcquisitionError, Dependency, IntoDependency, RegistryContainer, ResolvedDependency};
use crate::file_collection::FileCollection;
use crate::flow::shared::{Artifact, ImmutableArtifact};

#[derive(Debug)]
pub struct Configuration {
    inner: Arc<Mutex<ConfigurationInner>>
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
                registry_container: registry_container.clone()
            }))
        }
    }

    /// Create a of this configuration
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone()
        }
    }

    fn inner<R, F : FnOnce(&mut ConfigurationInner) -> R>(&self, func: F) -> R {
        let mut inner = self.inner.lock().unwrap();
        (func)(&mut inner)
    }

    fn inner_mut<R, F : FnOnce(&mut ConfigurationInner) -> R>(&mut self, func: F) -> R {
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
        where D::IntoDep : 'static
    {
        let dependency = dependency.into_dependency();
        self.inner_mut(move |dep| {
            dep.dependencies.push(Box::new(dependency))
        })
    }

    /// Adds a configuration that this configuration extends from
    pub fn extends_from(&mut self, other: &Configuration) {
        self.inner_mut(|inner| {
            inner.parents.push(other.clone());
        })
    }
}

struct ConfigurationInner {
    name: String,
    parents: Vec<Configuration>,
    dependencies: Vec<Box<dyn Dependency>>,
    resolved: OnceCell<ResolvedConfiguration>,

    registry_container: Arc<Mutex<RegistryContainer>>
}

impl ConfigurationInner {

    fn resolve(&mut self) -> Result<ResolvedConfiguration, AcquisitionError> {
        let dependencies = self.dependencies.drain(..).collect::<Vec<_>>();

        self.resolved
            .get_or_try_init(|| {
                let mut resolved = vec![];

                'outer:
                for dependency in dependencies {
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
                        return Err(AcquisitionError::from_iter(errors))
                    }
                }

                Ok(ResolvedConfiguration {
                    dependencies: resolved
                })
            })
            .map(|res| res.clone())
    }
}

impl Debug for ConfigurationInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("{}", self))
            .field("parents", &self.parents)
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
    dependencies: Vec<ResolvedDependency>
}

impl Display for ResolvedConfiguration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut list = f.debug_list();
        for dep in &self.dependencies {
            let artifacts = dep.artifacts()
                .into_iter()
                .map(|s| ImmutableArtifact::new(s))
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


