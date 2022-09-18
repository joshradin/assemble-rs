//! Provides implementations of providers

use crate::__export::{ProjectResult, TaskId};
use crate::lazy_evaluation::{IntoProvider, Provider};
use crate::project::buildable::Buildable;
use crate::Project;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::sync::Arc;

/// create a provider with a function
#[macro_export]
macro_rules! provider {
    ($e:expr) => {
        $crate::lazy_evaluation::providers::FnProvider::new($e)
    };
    ($e:expr) => {
        $crate::lazy_evaluation::providers::FnProvider::new($e)
    };
}

/// A provider created from a function
pub struct FnProvider<F, T, R>
where
    F: Fn() -> R + Send + Sync,
    R: Into<Option<T>>,
    T: Send + Sync + Clone,
{
    func: F,
    _phantom: PhantomData<T>
}

impl<F, T, R> Clone for FnProvider<F, T, R> where
    F: Fn() -> R + Send + Sync + Clone,
    R: Into<Option<T>>,
    T: Send + Sync + Clone, {
    fn clone(&self) -> Self {
        Self {
            func: self.func.clone(),
            _phantom: PhantomData
        }
    }
}

impl<F, T, R> Buildable for FnProvider<F, T, R>
where
    F: Fn() -> R + Send + Sync,
    R: Into<Option<T>>,
    T: Clone + Send + Sync,
{
    fn get_dependencies(&self, _: &Project) -> ProjectResult<HashSet<TaskId>> {
        Ok(HashSet::new())
    }
}

impl<F, T, R> Debug for FnProvider<F, T, R>
where
    F: Fn() -> R + Send + Sync,
    R: Into<Option<T>>,
    T: Clone + Send + Sync,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "FunctionalProvider")
    }
}

impl<F, T, R> Provider<T> for FnProvider<F, T, R>
where
    F: Fn() -> R + Send + Sync,
    R: Into<Option<T>>,
    T: Send + Sync + Clone,
{
    fn try_get(&self) -> Option<T> {
        (self.func)().into()
    }
}

impl<F, T, R> FnProvider<F, T, R>
where
    F: Fn() -> R + Send + Sync,
    R: Into<Option<T>>,
    T: Send + Sync + Clone,
{
    pub fn new(func: F) -> Self {
        Self { func, _phantom: PhantomData }
    }
}

/// Provides methods to map the output of a map to another
#[derive(Clone)]
pub struct Map<T, R, F, P>
where
    T: Send + Sync + Clone,
    R: Send + Sync + Clone,
    F: Fn(T) -> R + Send + Sync,
    P: Provider<T>,
{
    provider: P,
    transform: F,
    _data: PhantomData<(T, R)>,
}

impl<T, R, F, P> Buildable for Map<T, R, F, P> where F: Fn(T) -> R + Send + Sync, P: Provider<T>, R: Clone + Send + Sync, T: Clone + Send + Sync {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        self.provider.get_dependencies(project)
    }
}

impl<T, R, F, P> Debug for Map<T, R, F, P> where F: Fn(T) -> R + Send + Sync, P: Provider<T>, R: Clone + Send + Sync, T: Clone + Send + Sync {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Map<{:?}>", self.provider)
    }
}

impl<T, R, F, P> Provider<R> for Map<T, R, F, P>
where
    T: Send + Sync + Clone,
    R: Send + Sync + Clone,
    F: Fn(T) -> R + Send + Sync,
    P: Provider<T>,
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
    P: Provider<T>,
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
    PT: Provider<T>,
    PR: Provider<R>,
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
    PT: Provider<T>,
    PR: Provider<R>,
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

impl<T, R, PT, PR, F> Buildable for FlatMap<T, R, PT, PR, F> where F: Fn(T) -> PR + Send + Sync, PR: Provider<R>, PT: Provider<T>, R: Clone + Send + Sync, T: Clone + Send + Sync {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        self.provider.get_dependencies(project)
    }
}

impl<T, R, PT, PR, F> Debug for FlatMap<T, R, PT, PR, F> where F: Fn(T) -> PR + Send + Sync, PR: Provider<R>, PT: Provider<T>, R: Clone + Send + Sync, T: Clone + Send + Sync {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "FlatMap<{:?}>", self.provider)
    }
}

impl<T, R, PT, PR, F> Provider<R> for FlatMap<T, R, PT, PR, F>
where
    T: Send + Sync + Clone,
    R: Send + Sync + Clone,
    PT: Provider<T>,
    PR: Provider<R>,
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
    left: Arc<dyn Provider<T>>,
    right: Arc<dyn Provider<B>>,
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

impl<T, B, R, F> Buildable for Zip<T, B, R, F> where B: Clone + Send + Sync, F: Fn(T, B) -> R + Send + Sync, R: Clone + Send + Sync, T: Clone + Send + Sync {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        self.left.get_dependencies(project)
    }
}

impl<T, B, R, F> Debug for Zip<T, B, R, F> where B: Clone + Send + Sync, F: Fn(T, B) -> R + Send + Sync, R: Clone + Send + Sync, T: Clone + Send + Sync {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<T, B, R, F> Provider<R> for Zip<T, B, R, F>
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

impl<T: Send + Sync + Clone + Debug, F: Send + FnOnce() -> T> Buildable for Lazy<T, F> {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        Ok(HashSet::new())
    }
}


impl<T: Send + Sync + Clone + Debug> Buildable for Option<T> {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        Ok(HashSet::new())
    }
}

impl<T: Send + Sync + Clone + Debug> Provider<T> for Option<T> {
    fn try_get(&self) -> Option<T> {
        self.clone()
    }
}


impl<T: Send + Sync + Clone + Debug, E: Send + Sync + Debug> Buildable for Result<T, E> {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        Ok(HashSet::new())
    }
}

impl<T: Send + Sync + Clone + Debug, E: Send + Sync + Debug> Provider<T> for Result<T, E> {
    fn try_get(&self) -> Option<T> {
        self.as_ref().ok().cloned()
    }
}

impl<T: Send + Sync + Clone + Debug, F: Send + FnOnce() -> T> Provider<T> for Lazy<T, F> {
    fn try_get(&self) -> Option<T> {
        Some(Lazy::force(self).clone())
    }
}

/// Used to flatten providers
pub type Flatten<T, B, P> = FlatMap<T, B, P, T, fn(T) -> T>;
