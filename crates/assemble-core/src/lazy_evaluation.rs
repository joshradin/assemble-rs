//! Lazy evaluation allows for a more simple approach to sharing data between tasks.
//!
//! The main driving trait that allows for this is [`Provider`](Provider). Objects that
//! implement this trait can try to provide a value of that type. There are three main flavors
//! of providers.
//!
//! Properties
//! ---
//!
//! `Prop<T>` - A property that can be set to a specific value.
//!
//! ## Usage
//! Should be used when single values are needed, and this property is needed to either be used
//! as an output that's set by the task, or the task needs an input. When used with the
//! [`CreateTask`](crate::tasks::CreateTask) derive macro.
//! ```
//! # use assemble_core::lazy_evaluation::Prop;
//! struct LogTask {
//!     msg: Prop<String>
//! }
//! ```
//!
//!
//!
//!

pub mod anonymous;
pub mod prop;
pub mod providers;

use crate::lazy_evaluation::providers::{FlatMap, Flatten, Map, Zip};
use crate::Project;
use crate::__export::{ProjectResult, TaskId};
use crate::project::buildable::Buildable;
pub use prop::*;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

/// The provider trait represents an object that can continuously produce a value. Provider values
/// can be chained together using the [`ProviderExt`][0] trait.
///
/// For convenience, several types from the default library implement this trait.
/// - `for<T> Option<T> : Provider<T>`
/// - `for<F, T> F : Provider<T> where F : Fn() -> T`
///
/// [0]: ProviderExt
pub trait Provider<T: Clone + Send + Sync>: Send + Sync + Buildable {
    /// The missing message for this provider
    fn missing_message(&self) -> String {
        String::from("Provider has no value set")
    }
    /// Get a value from the provider.
    ///
    /// # Panic
    /// This method will panic if there is no value available.
    ///
    /// # Example
    /// ```
    /// # use assemble_core::lazy_evaluation::Provider;
    /// let prop = || 10;
    /// assert_eq!(prop.get(), 10);
    /// ```
    fn get(&self) -> T {
        self.try_get().expect(&self.missing_message())
    }

    /// Try to get a value from the provider.
    ///
    /// Will return `Some(v)` if value `v` is available, otherwise `None` is returned.
    ///
    /// # Example
    /// ```
    /// # use assemble_core::lazy_evaluation::Provider;
    /// let prop = || 10;
    /// assert_eq!(prop.try_get(), Some(10));
    ///
    /// // options implement provider
    /// let prop = Option::<usize>::None;
    /// assert_eq!(prop.try_get(), None);
    /// ```
    fn try_get(&self) -> Option<T>;

    /// Tries to get a value from this provider, returning an error if not available.
    ///
    /// The error's message is usually specified by the `missing_message()` method.
    ///
    /// # Example
    /// ```
    /// # use assemble_core::identifier::Id;
    /// # use assemble_core::lazy_evaluation::Provider;
    /// use assemble_core::lazy_evaluation::Prop;
    /// use assemble_core::lazy_evaluation::anonymous::AnonymousProvider;
    ///
    /// let provider = || 3_i32;
    /// assert!(matches!(provider.fallible_get(), Ok(3)));
    /// let mut prop = Prop::<i32>::new(Id::from("test"));
    /// assert!(matches!(prop.fallible_get(), Err(_)));
    /// prop.set_with(provider).unwrap();
    /// assert!(matches!(prop.fallible_get(), Ok(3)));
    /// ```
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
    fn flatten<B>(self) -> Flatten<T, B, Self>
    /* FlatMap<T, B, Self, T, fn(T) -> T> */
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

impl<T: Clone + Send + Sync> Buildable for Wrapper<T> {
    fn get_dependencies(&self, _: &Project) -> ProjectResult<HashSet<TaskId>> {
        Ok(HashSet::new())
    }
}

impl<T: Clone + Send + Sync> Debug for Wrapper<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Wrapper").finish_non_exhaustive()
    }
}

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
    use crate::provider;

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
        let flap_map = provider.clone().flat_map(|v| provider!(move || v * v));
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
        let prop1 = provider!(|| provider!(|| 5));
        let prop2 = provider!(|| provider!(|| 6));
        let value = prop1.flatten().zip(prop2.flatten(), |l, r| l * r);
        assert_eq!(value.get(), 30);

        let flattend2 = AnonymousProvider::with_value(|| 0).flat_map(|p| provider!(p));
    }
}
