//! Anonymous properties.

use crate::__export::{ProjectResult, TaskId};
use crate::lazy_evaluation::{IntoProvider, Provider, ProviderError};
use crate::project::buildable::{Buildable, BuiltByContainer, IntoBuildable};
use crate::{provider, Project};
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};

use std::sync::Arc;

/// An anonymous prop is used to store provided values without needing an
/// identifier
#[derive(Clone)]
pub struct AnonymousProvider<T: Clone + Send + Sync> {
    inner: Arc<dyn Provider<T>>,
    /// allow for extra built by definitions
    extra_built_by: BuiltByContainer,
}

impl<T: Clone + Send + Sync> Buildable for AnonymousProvider<T> {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        Ok(self
            .inner
            .get_dependencies(project)?
            .into_iter()
            .chain(self.extra_built_by.get_dependencies(project)?)
            .collect())
    }
}

impl<T: Clone + Send + Sync> Provider<T> for AnonymousProvider<T> {
    fn missing_message(&self) -> String {
        self.inner.missing_message()
    }

    fn get(&self) -> T {
        self.inner.get()
    }

    fn try_get(&self) -> Option<T> {
        self.inner.try_get()
    }

    fn fallible_get(&self) -> Result<T, ProviderError> {
        self.inner.fallible_get()
    }
}

impl<T: Clone + Send + Sync> Debug for AnonymousProvider<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnonymousProp").finish_non_exhaustive()
    }
}

impl<T: Clone + Send + Sync> AnonymousProvider<T> {
    pub fn new<P: IntoProvider<T>>(provider: P) -> Self
    where
        <P as IntoProvider<T>>::Provider: 'static,
    {
        let provider = provider.into_provider();
        let boxed = Arc::new(provider) as Arc<dyn Provider<T>>;
        Self {
            inner: boxed,
            extra_built_by: BuiltByContainer::new(),
        }
    }

    pub fn with_value(val: T) -> Self
    where
        T: 'static,
    {
        let boxed = Arc::new(provider!(move || val.clone())) as Arc<dyn Provider<T>>;
        Self {
            inner: boxed,
            extra_built_by: BuiltByContainer::new(),
        }
    }

    /// Adds something that builds this provider
    pub fn built_by<B: IntoBuildable>(mut self, buildable: B) -> Self
    where
        <B as IntoBuildable>::Buildable: 'static,
    {
        self.extra_built_by.add(buildable);
        self
    }
}
