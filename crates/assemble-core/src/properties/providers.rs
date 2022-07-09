//! Provides implementations of providers

use std::marker::PhantomData;
use std::sync::Arc;
use crate::properties::{AsProvider, Provides};

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
pub struct Map<T, R, F>
where
    T: Send + Sync + Clone,
    R: Send + Sync + Clone,
    F: Fn(T) -> R + Send + Sync,
{
    provider: Arc<dyn Provides<T>>,
    transform: F,
}

impl<'t, T, R, F> Provides<R> for Map<T, R, F> where
    T: Send + Sync + Clone,
    R: Send + Sync + Clone,
    F: Fn(T) -> R + Send + Sync, {
    fn try_get(&self) -> Option<R> {
        self.provider
            .try_get()
            .map(|v| (self.transform)(v))
    }
}

impl<T, R, F> Map<T, R, F> where
    T: Send + Sync + Clone,
    R: Send + Sync + Clone,
    F: Fn(T) -> R + Send + Sync, {
    pub(in super) fn new<P : AsProvider<T>>(provider: &P, transform: F) -> Self
        where <P as AsProvider<T>>::Provider : 'static
    {
        Self { provider: Arc::new(provider.as_provider()), transform }
    }
}

/// Provides methods to map the output of a map to another
#[derive(Clone)]
pub struct FlatMap<T, R, P, F>
    where
        T: Send + Sync + Clone,
        R: Send + Sync + Clone,
        P: Provides<R>,
        F: Fn(T) -> P+ Send + Sync,
{
    provider: Arc<dyn Provides<T>>,
    transform: F,
    _data: PhantomData<R>
}

impl<T, R, P, F> FlatMap<T, R, P, F> where
    T: Send + Sync + Clone,
    R: Send + Sync + Clone,
    P: Provides<R>,
    F: Fn(T) -> P + Send + Sync, {
    pub(in super) fn new<Pr : AsProvider<T>>(provider: &Pr, transform: F) -> Self
        where <Pr as AsProvider<T>>::Provider : 'static {
        Self { provider: Arc::new(provider.as_provider()), transform, _data: PhantomData }
    }
}

impl<T, R, P, F> Provides<R> for FlatMap<T, R, P, F> where
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

#[derive(Clone)]
pub struct Zip<T, B, R, F>
where
    T: Send + Sync + Clone,
    B: Send + Sync + Clone,
    R: Send + Sync + Clone,
    F: Fn(T, B) -> R + Send + Sync
{
    left: Arc<dyn Provides<T>>,
    right: Arc<dyn Provides<B>>,
    transform: F
}

impl<T, B, R, F> Zip<T, B, R, F> where
    T: Send + Sync + Clone,
    B: Send + Sync + Clone,
    R: Send + Sync + Clone,
    F: Fn(T, B) -> R + Send + Sync {

    pub fn new<PL, PR>(left: &PL, right: &PR, zip_func: F) -> Self
        where PL: AsProvider<T>,
            <PL as AsProvider<T>>::Provider : 'static,
              PR: AsProvider<B>,
    <PR as AsProvider<B>>::Provider : 'static,{
        Self {
            left: Arc::new(left.as_provider()),
            right: Arc::new(right.as_provider()),
            transform: zip_func
        }
    }

}

impl<T, B, R, F> Provides<R> for Zip<T, B, R, F> where
    T: Send + Sync + Clone,
    B: Send + Sync + Clone,
    R: Send + Sync + Clone,
    F: Fn(T, B) -> R + Send + Sync {
    fn try_get(&self) -> Option<R> {
        let left = self.left.try_get();
        let right = self.right.try_get();

        left.zip(right)
            .map(|(l, r)| (self.transform)(l, r))
    }
}