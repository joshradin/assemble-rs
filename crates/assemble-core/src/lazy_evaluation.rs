//! Implementation of Properties 2.0. Ideally, this should allow for improved inter task sharing

pub mod anonymous;
pub mod prop;
pub mod providers;

use crate::lazy_evaluation::providers::{FlatMap, Flatten, Map, Zip};
use crate::Project;
pub use prop::*;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

/// The provider trait represents an object that can continuously produce a value. Provider values
/// can be chained together using the [`ProviderExt`][0] trait.
///
/// [0]: ProviderExt
pub trait Provider<T: Clone + Send + Sync>: Send + Sync {
    /// The missing message for this provider
    fn missing_message(&self) -> String {
        String::from("Provider has no value set")
    }
    /// Get a value from the provider
    fn get(&self) -> T {
        self.try_get().expect(&self.missing_message())
    }

    /// Try to get a value from the provider
    fn try_get(&self) -> Option<T>;

    /// Same as try_get, but returns a Result
    fn fallible_get(&self) -> Result<T, ProviderError> {
        self.try_get()
            .ok_or_else(|| ProviderError::new(self.missing_message()))
    }
}

assert_obj_safe!(Provider<()>);

/// Provides extensions that are not object safe to the Provider trait.
pub trait ProviderExt<T: Clone + Send + Sync>: Provider<T> + Sized {
    /// Creates a provider that can map the output of one provider into some other value.
    ///
    /// `transform`: `fn(T) -> R`
    fn map<R, F>(self, transform: F) -> Map<T, R, F, Self>
    where
        R: Send + Sync + Clone,
        F: Fn(T) -> R + Send + Sync,
        Self: 'static,
    {
        Map::new(self, transform)
    }

    /// Creates a provider that can map the output of one provider with type `T` into some other value that's
    /// also a provider of type `R`. The created provider is a provider of type `R`
    ///
    /// `transform`: `fn(T) -> impl Provider<R>`
    fn flat_map<R, P, F>(self, transform: F) -> FlatMap<T, R, Self, P, F>
    where
        R: Send + Sync + Clone,
        P: Provider<R>,
        F: Fn(T) -> P + Send + Sync,
    {
        FlatMap::new(self, transform)
    }

    /// Flattens a provider that provides another provider
    fn flatten<B>(self) -> Flatten<T, B, Self> /* FlatMap<T, B, Self, T, fn(T) -> T> */
    where
        Self: Clone + 'static,
        T: Provider<B>,
        B: Clone + Send + Sync,
    {
        self.flat_map(|s| s)
    }

    /// Creates a provider that can map the output of two provider with type `T1` and `T2` into some other value
    /// and transforms the two values into one output value of type `B`.
    ///
    /// `transform`: `fn(T1, T2) -> B`
    fn zip<P, B, R, F>(self, other: P, func: F) -> Zip<T, B, R, F>
    where
        Self: 'static,
        P: IntoProvider<B>,
        <P as IntoProvider<B>>::Provider: 'static,
        B: Send + Sync + Clone,
        R: Send + Sync + Clone,
        F: Fn(T, B) -> R + Send + Sync,
    {
        Zip::new(self, other, func)
    }
}

impl<P, T> ProviderExt<T> for P
where
    T: Clone + Send + Sync,
    P: Provider<T> + Send + Sync + 'static,
{
}

/// Represents that a type can be represented as a provider. All Providers implement this trait.
pub trait IntoProvider<T: Send + Sync + Clone> {
    type Provider: Provider<T>;

    fn into_provider(self) -> Self::Provider;
}

impl<P, T> IntoProvider<T> for P
where
    T: Clone + Send + Sync,
    P: Provider<T> + Send + Sync,
{
    type Provider = Self;

    fn into_provider(self) -> Self::Provider {
        self
    }
}

#[derive(Clone)]
struct Wrapper<T: Clone + Send + Sync>(T);

impl<T: Clone + Send + Sync> Provider<T> for Wrapper<T> {
    fn try_get(&self) -> Option<T> {
        Some(self.0.clone())
    }
}

/// A value could not be provided
#[derive(Debug, thiserror::Error)]
#[error("{}", message)]
pub struct ProviderError {
    message: String,
}

impl ProviderError {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lazy_evaluation::anonymous::AnonymousProvider;
    use crate::lazy_evaluation::IntoProvider;

    #[test]
    fn map() {
        let mut provider = Prop::with_value(5);
        let mapped = provider.clone().map(|v| v * v);
        assert_eq!(mapped.get(), 25);
        provider.set(10).unwrap();
        assert_eq!(mapped.get(), 100);
    }

    #[test]
    fn flat_map() {
        let mut provider = Prop::with_value(5u32);
        let flap_map = provider.clone().flat_map(|v| move || v * v);
        assert_eq!(flap_map.get(), 25);
        provider.set(10u32).unwrap();
        assert_eq!(flap_map.get(), 100);
    }

    #[test]
    fn zip() {
        let mut provider_l = Prop::with_value(5);
        let mut provider_r = Prop::with_value(6);
        let zipped = provider_l.clone().zip(provider_r.clone(), |l, r| l * r);
        assert_eq!(zipped.get(), 30);
        provider_l.set(10).unwrap();
        assert_eq!(zipped.get(), 60);
        provider_r.set(15).iter();
        assert_eq!(zipped.get(), 150);
    }

    #[test]
    fn flatten() {
        let prop1 = || || 5;
        let prop2 = || || 6;
        let value = prop1.flatten().zip(prop2.flatten(), |l, r| l * r);
        assert_eq!(value.get(), 30);

        let flattend2 = AnonymousProvider::with_value(|| 0).flat_map(|p| p);
    }
}