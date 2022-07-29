//! Provides the configuration type, which is a live type that can extend other configurations

use crate::utilities::Shared;

#[derive(Clone)]
pub struct Configuration {
    name: String,
    inner: Shared<ConfigurationInner>
}

impl Configuration {

    fn with_inner<F, R>(&self, func: F) -> R
        where F : FnOnce(&mut ConfigurationInner) -> R
    {
        let mut guard = self.inner.lock().unwrap();
        (func)(&mut *guard)
    }

    pub fn extends(&mut self, other: &Configuration) {
        self.with_inner(|inner| {
            inner.parents.push(other.clone());
        });
    }
}

struct ConfigurationInner {
    parents: Vec<Configuration>,
}