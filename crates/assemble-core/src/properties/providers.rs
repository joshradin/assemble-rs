//! Provides implementations of providers

use crate::properties::{IntoProvider, Provides};
use std::marker::PhantomData;
use std::sync::Arc;

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
#[derive(Clone)]
pub struct Map<T, R, F, P>
where
    T: Send + Sync + Clone,
    R: Send + Sync + Clone,
    F: Fn(T) -> R + Send + Sync,
    P: Provides<T>,
{
    provider: P,
    transform: F,
    _data: PhantomData<(T, R)>,
}

impl<T, R, F, P> Provides<R> for Map<T, R, F, P>
where
    T: Send + Sync + Clone,
    R: Send + Sync + Clone,
    F: Fn(T) -> R + Send + Sync,
    P: Provides<T>,
{
    fn missing_message(&self) -> String {
        self.provider.missing_message()
    }

    fn try_get(&self) -> Option<R> {
        self.provider.try_get().map(|v| (self.transform)(v))
    }
}

impl<T, R, F, P> Map<T, R, F, P>
where
    T: Send + Sync + Clone,
    R: Send + Sync + Clone,
    F: Fn(T) -> R + Send + Sync,
    P: Provides<T>,
{
    pub(super) fn new(provider: P, transform: F) -> Self {
        Self {
            provider,
            transform,
            _data: Default::default(),
        }
    }
}

/// Provides methods to map the output of a map to another
#[derive(Clone)]
pub struct FlatMap<T, R, PT, PR, F>
where
    T: Send + Sync + Clone,
    R: Send + Sync + Clone,
    PT: Provides<T>,
    PR: Provides<R>,
    F: Fn(T) -> PR + Send + Sync,
{
    provider: PT,
    transform: F,
    _data: PhantomData<(R, T, PR)>,
}

impl<T, R, PR, PT, F> FlatMap<T, R, PT, PR, F>
where
    T: Send + Sync + Clone,
    R: Send + Sync + Clone,
    PT: Provides<T>,
    PR: Provides<R>,
    F: Fn(T) -> PR + Send + Sync,
{
    pub(super) fn new(provider: PT, transform: F) -> Self {
        Self {
            provider,
            transform,
            _data: PhantomData,
        }
    }
}

impl<T, R, PT, PR, F> Provides<R> for FlatMap<T, R, PT, PR, F>
where
    T: Send + Sync + Clone,
    R: Send + Sync + Clone,
    PT: Provides<T>,
    PR: Provides<R>,
    F: Fn(T) -> PR + Send + Sync,
{
    fn missing_message(&self) -> String {
        self.provider.missing_message()
    }

    fn get(&self) -> R {
        let start = self
            .provider
            .try_get()
            .expect(&self.provider.missing_message());
        let transformed = (self.transform)(start);
        transformed.try_get().expect(&transformed.missing_message())
    }

    fn try_get(&self) -> Option<R> {
        self.provider
            .try_get()
            .and_then(|gotten| (self.transform)(gotten).try_get())
    }
}

#[derive(Clone)]
pub struct Zip<T, B, R, F>
where
    T: Send + Sync + Clone,
    B: Send + Sync + Clone,
    R: Send + Sync + Clone,
    F: Fn(T, B) -> R + Send + Sync,
{
    left: Arc<dyn Provides<T>>,
    right: Arc<dyn Provides<B>>,
    transform: F,
}

impl<T, B, R, F> Zip<T, B, R, F>
where
    T: Send + Sync + Clone,
    B: Send + Sync + Clone,
    R: Send + Sync + Clone,
    F: Fn(T, B) -> R + Send + Sync,
{
    pub fn new<PL, PR>(left: PL, right: PR, zip_func: F) -> Self
    where
        PL: IntoProvider<T>,
        <PL as IntoProvider<T>>::Provider: 'static,
        PR: IntoProvider<B>,
        <PR as IntoProvider<B>>::Provider: 'static,
    {
        Self {
            left: Arc::new(left.into_provider()),
            right: Arc::new(right.into_provider()),
            transform: zip_func,
        }
    }
}

impl<T, B, R, F> Provides<R> for Zip<T, B, R, F>
where
    T: Send + Sync + Clone,
    B: Send + Sync + Clone,
    R: Send + Sync + Clone,
    F: Fn(T, B) -> R + Send + Sync,
{
    fn missing_message(&self) -> String {
        format!(
            "{} or {}",
            self.left.missing_message(),
            self.right.missing_message()
        )
    }

    fn try_get(&self) -> Option<R> {
        let left = self.left.try_get();
        let right = self.right.try_get();

        left.zip(right).map(|(l, r)| (self.transform)(l, r))
    }
}

impl<T: Send + Sync + Clone> Provides<T> for Option<T> {
    fn try_get(&self) -> Option<T> {
        self.clone()
    }
}

impl<T: Send + Sync + Clone, E: Send + Sync> Provides<T> for Result<T, E> {
    fn try_get(&self) -> Option<T> {
        self.as_ref().ok().cloned()
    }
}
