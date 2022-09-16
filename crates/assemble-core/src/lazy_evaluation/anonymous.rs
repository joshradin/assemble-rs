//! Anonymous properties.

use crate::lazy_evaluation::{IntoProvider, Provider, ProviderError};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::sync::Arc;

/// An anonymous prop is used to store provided values without needing an
/// identifier
#[derive(Clone)]
pub struct AnonymousProvider<T: Clone + Send + Sync> {
    inner: Arc<dyn Provider<T>>,
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
        Self { inner: boxed }
    }

    pub fn with_value(val: T) -> Self
    where
        T: 'static,
    {
        let boxed = Arc::new(move || val.clone()) as Arc<dyn Provider<T>>;
        Self { inner: boxed }
    }
}
