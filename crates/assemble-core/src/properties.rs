//! Implementation of Properties 2.0. Ideally, this should allow for improved inter task sharing

mod prop;
use crate::properties::providers::{FlatMap, Map};
pub use prop::*;

pub mod providers;


pub trait Provides<T: Clone + Send + Sync>: Send + Sync {
    fn get(&self) -> T {
        self.try_get().unwrap()
    }
    fn try_get(&self) -> Option<T>;


}

assert_obj_safe!(Provides<()>);

pub trait ProvidesExt<T: Clone + Send + Sync> : Provides<T> + Sized {
    fn map<R, F>(&self, transform: F) -> Map<T, R, F>
        where
            R: Send + Sync + Clone,
            F: Fn(T) -> R + Send + Sync,
    {
        Map::new(self, transform)
    }

    fn flat_map<R, P, F>(&self, transform: F) -> FlatMap<T, R, P, F>
        where
            R: Send + Sync + Clone,
            P: Provides<R>,
            F: Fn(T) -> P + Send + Sync,
    {
        FlatMap::new(self, transform)
    }
}

impl<P, T>ProvidesExt<T> for P
    where
        T: Clone + Send + Sync,
        P: Provides<T> + Send + Sync + Clone,
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


#[cfg(test)]
mod tests {
    use crate::properties::AsProvider;
    use super::*;

    #[test]
    fn flat_map() {

        let provider = Wrapper(5u32);
        let flap_map = provider.flat_map(|v| move || v*v);
    }
}