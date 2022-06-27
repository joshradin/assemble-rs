//! Provides implementations of providers

use std::marker::PhantomData;
use crate::properties::Provides;

impl<T, F, R> Provides<T> for F
where
    F: Send + Sync,
    F: Fn() -> R,
    Option<T>: From<R>,
    T: Send + Sync + Clone,
{
    fn try_get(&self) -> Option<T> {
        let output: R = (self)();
        Option::from(output)
    }
}

/// Provides methods to map the output of a map to another
pub struct Map<'t, T, R, F>
where
    T: Send + Sync + Clone,
    R: Send + Sync + Clone,
    F: Fn(T) -> R + Send + Sync,
{
    provider: &'t dyn Provides<T>,
    transform: F,
}

impl<'t, T, R, F> Provides<R> for Map<'t, T, R, F> where
    T: Send + Sync + Clone,
    R: Send + Sync + Clone,
    F: Fn(T) -> R + Send + Sync, {
    fn try_get(&self) -> Option<R> {
        self.provider
            .try_get()
            .map(|v| (self.transform)(v))
    }
}

impl<'t, T, R, F> Map<'t, T, R, F> where
    T: Send + Sync + Clone,
    R: Send + Sync + Clone,
    F: Fn(T) -> R + Send + Sync, {
    pub(in super) fn new(provider: &'t dyn Provides<T>, transform: F) -> Self {
        Self { provider, transform }
    }
}

/// Provides methods to map the output of a map to another
pub struct FlatMap<'t, T, R, P, F>
    where
        T: Send + Sync + Clone,
        R: Send + Sync + Clone,
        P: Provides<R>,
        F: Fn(T) -> P+ Send + Sync,
{
    provider: &'t dyn Provides<T>,
    transform: F,
    _data: PhantomData<R>
}

impl<'t, T, R, P, F> FlatMap<'t, T, R, P, F> where
    T: Send + Sync + Clone,
    R: Send + Sync + Clone,
    P: Provides<R>,
    F: Fn(T) -> P + Send + Sync, {
    pub(in super) fn new(provider: &'t dyn Provides<T>, transform: F) -> Self {
        Self { provider, transform, _data: PhantomData }
    }
}

impl<'t, T, R, P, F> Provides<R> for FlatMap<'t, T, R, P, F> where
    T: Send + Sync + Clone,
    R: Send + Sync + Clone,
    P: Provides<R>,
    F: Fn(T) -> P + Send + Sync, {
    fn try_get(&self) -> Option<R> {
        self.provider
            .try_get()
            .and_then(
                |gotten| (self.transform)(gotten).try_get()
            )
    }
}

