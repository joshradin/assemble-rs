//! A configuration has two states: resolved and unresolved

use std::fmt::{Debug, Display, Formatter, write};
use std::sync::{Arc, Mutex};
use once_cell::sync::OnceCell;
use crate::dependencies::{AcquisitionError, Dependency, RegistryContainer, ResolvedDependency};

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
    pub fn add_dependency<D: Dependency + 'static>(&mut self, dependency: D) {
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
                for dependency in dependencies {
                    let registry_c = self.registry_container.lock().unwrap();

                    let mut found = false;
                    for registry in registry_c.supported_registries(&dependency.dep_type()) {
                        if let Ok(resolved_dep) = dependency.try_resolve(registry) {
                            resolved.push(resolved_dep);
                            found = true;
                            break;
                        }
                    }

                    if !found {
                        return Err(AcquisitionError::custom(format!("couldn't download dependency {}", dependency.id())))
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