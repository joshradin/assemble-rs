//! Implementation of Properties 2.0. Ideally, this should allow for improved inter task sharing

mod prop;
use crate::properties::providers::{FlatMap, Map, Zip};
pub use prop::*;

pub mod providers;
pub mod task_properties;

use crate::Project;
use task_properties::TaskProperties;

pub trait Provides<T: Clone + Send + Sync>: Send + Sync {
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
}

assert_obj_safe!(Provides<()>);

pub trait ProvidesExt<T: Clone + Send + Sync>: Provides<T> + Clone + Sized {
    fn map<R, F>(&self, transform: F) -> Map<T, R, F>
    where
        R: Send + Sync + Clone,
        F: Fn(T) -> R + Send + Sync,
        Self: 'static,
    {
        Map::new(self, transform)
    }

    fn flat_map<R, P, F>(self, transform: F) -> FlatMap<T, R, Self, P, F>
    where
        R: Send + Sync + Clone,
        P: Provides<R>,
        F: Fn(T) -> P + Send + Sync,
    {
        FlatMap::new(self, transform)
    }

    fn zip<P, B, R, F>(&self, other: &P, func: F) -> Zip<T, B, R, F>
    where
        P: AsProvider<B> + Send + Sync,
        <P as AsProvider<B>>::Provider: 'static,
        B: Send + Sync + Clone,
        R: Send + Sync + Clone,
        F: Fn(T, B) -> R + Send + Sync,
        Self: 'static,
    {
        Zip::new(self, other, func)
    }
}

impl<P, T> ProvidesExt<T> for P
where
    T: Clone + Send + Sync,
    P: Provides<T> + Send + Sync + Clone + 'static,
{
}

/// Represents that a type can be represented as a provider. All Providers implement this trait.
pub trait AsProvider<T: Send + Sync + Clone> {
    type Provider: Provides<T>;

    fn as_provider(&self) -> Self::Provider;
}

impl<P, T> AsProvider<T> for P
where
    T: Clone + Send + Sync,
    P: Provides<T> + Send + Sync + Clone,
{
    type Provider = Self;

    fn as_provider(&self) -> Self::Provider {
        self.clone()
    }
}

#[derive(Clone)]
struct Wrapper<T: Clone + Send + Sync>(T);

impl<T: Clone + Send + Sync> Provides<T> for Wrapper<T> {
    fn try_get(&self) -> Option<T> {
        Some(self.0.clone())
    }
}

pub trait FromProperties {
    fn from_properties(properties: &mut TaskProperties, project: &Project) -> Self;
    fn from_properties_projectless(properties: &mut TaskProperties) -> Self;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::properties::AsProvider;

    #[test]
    fn map() {
        let mut provider = Prop::with_value(5);
        let mapped = provider.map(|v| v * v);
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
        let zipped = provider_l.zip(&provider_r, |l, r| l * r);
        assert_eq!(zipped.get(), 30);
        provider_l.set(10).unwrap();
        assert_eq!(zipped.get(), 60);
        provider_r.set(15).iter();
        assert_eq!(zipped.get(), 150);
    }
}
