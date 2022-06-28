use std::any::Any;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::DerefMut;
use std::panic::{catch_unwind, UnwindSafe};
use std::sync::{Arc, Mutex, RwLock};

pub trait AsAny {
    fn as_any(&self) -> &(dyn Any + '_);
}

pub trait AsAnyMut: 'static + AsAny {
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub fn move_from_deref<T: Default, O: DerefMut<Target = T>>(mut obj: O) -> T {
    let mutable = obj.deref_mut();
    let mut output = T::default();
    std::mem::swap(mutable, &mut output);
    output
}

pub trait ArcExt {
    type Consume;

    fn consume(self) -> Self::Consume;
}

impl<T: Clone> ArcExt for Arc<Mutex<T>> {
    type Consume = T;

    /// Creates a clone of the current state of the object in the Mutex
    fn consume(self) -> T {
        self.lock().unwrap().clone()
    }
}

impl<T: Clone> ArcExt for Arc<RwLock<T>> {
    type Consume = T;

    fn consume(self) -> Self::Consume {
        self.read().unwrap().clone()
    }
}

pub trait Spec<T: ?Sized> {
    fn accept(&self, value: &T) -> bool;
}

impl<T: ?Sized, S : Spec<T> + ?Sized> Spec<T> for Box<S> {
    fn accept(&self, value: &T) -> bool {
        (**self).accept(value)
    }
}

impl<T: ?Sized, S : Spec<T> + ?Sized> Spec<T> for Arc<S> {
    fn accept(&self, value: &T) -> bool {
        (**self).accept(value)
    }
}

impl<T : ?Sized, S : Spec<T>> Spec<T> for &S {
    fn accept(&self, value: &T) -> bool {
        (*self).accept(value)
    }
}

pub struct True<T: ?Sized>(PhantomData<T>);

impl<T: ?Sized> True<T> {
    pub fn new() -> Self {
        Self(PhantomData::default())
    }
}

impl<T: ?Sized> Spec<T> for True<T> {
    fn accept(&self, _value: &T) -> bool {
        true
    }
}

#[derive(Default)]
pub struct False<T: ?Sized>(PhantomData<T>);
impl<T: ?Sized> False<T> {
    pub fn new() -> Self {
        Self(PhantomData::default())
    }
}

impl<T: ?Sized> Spec<T> for False<T> {
    fn accept(&self, _value: &T) -> bool {
        false
    }
}

pub struct Invert<T: ?Sized, F: Spec<T>> {
    func: F,
    _phantom: PhantomData<T>,
}

impl<T: ?Sized, F: Spec<T>> Invert<T, F> {
    pub fn new(func: F) -> Self {
        Self {
            func,
            _phantom: Default::default(),
        }
    }
}

pub struct AndSpec<T: ?Sized, F1: Spec<T>, F2: Spec<T>> {
    func1: F1,
    func2: F2,
    _type: PhantomData<T>,
}

impl<T: ?Sized, F1: Spec<T>, F2: Spec<T>> AndSpec<T, F1, F2> {
    pub fn new(func1: F1, func2: F2) -> Self {
        Self {
            func1,
            func2,
            _type: Default::default(),
        }
    }
}

impl<T: ?Sized, F1: Spec<T>, F2: Spec<T>> Spec<T> for AndSpec<T, F1, F2> {
    fn accept(&self, value: &T) -> bool {
        self.func1.accept(value) && self.func2.accept(value)
    }
}

pub struct OrSpec<T: ?Sized, F1: Spec<T>, F2: Spec<T>> {
    func1: F1,
    func2: F2,
    _type: PhantomData<T>,
}

impl<T: ?Sized, F1: Spec<T>, F2: Spec<T>> OrSpec<T, F1, F2> {
    pub fn new(func1: F1, func2: F2) -> Self {
        Self {
            func1,
            func2,
            _type: Default::default(),
        }
    }
}

impl<T: ?Sized, F1: Spec<T>, F2: Spec<T>> Spec<T> for OrSpec<T, F1, F2> {
    fn accept(&self, value: &T) -> bool {
        self.func1.accept(value) || self.func2.accept(value)
    }
}


pub fn try_<T, R, F>(func: F) -> Result<T, R>
    where
        F : FnOnce() -> Result<T, R> + UnwindSafe,
        R : From<Box<dyn Any + Send + 'static>>
{
    match catch_unwind(func) {
        Ok(o) => { o }
        Err(e) => {
            Err(e.into())
        }
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Work {
    UpToDate,
    Skipped,
    Failed,
    Success
}