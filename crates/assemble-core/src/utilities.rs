use std::any::{Any, TypeId};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::DerefMut;
use std::panic::{catch_unwind, UnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock, Weak};
use std::time::Instant;
use parking_lot::{RwLockReadGuard, RwLockWriteGuard};

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

impl<T: ?Sized, S: Spec<T> + ?Sized> Spec<T> for Box<S> {
    fn accept(&self, value: &T) -> bool {
        (**self).accept(value)
    }
}

impl<T: ?Sized, S: Spec<T> + ?Sized> Spec<T> for Arc<S> {
    fn accept(&self, value: &T) -> bool {
        (**self).accept(value)
    }
}

impl<T: ?Sized, S: Spec<T>> Spec<T> for &S {
    fn accept(&self, value: &T) -> bool {
        (*self).accept(value)
    }
}

/// Create a callback
pub struct Callback<F, T: ?Sized>
where
    F: Fn(&T) -> bool,
{
    func: F,
    _data: PhantomData<T>,
}

impl<F, T: ?Sized> Callback<F, T>
where
    F: Fn(&T) -> bool,
{
    /// Create a new callback from a function
    fn new(func: F) -> Self {
        Self {
            func,
            _data: PhantomData,
        }
    }
}

impl<F, T: ?Sized> Spec<T> for Callback<F, T>
where
    F: Fn(&T) -> bool,
{
    fn accept(&self, value: &T) -> bool {
        (self.func)(value)
    }
}

/// Create a spec using a callback
pub fn spec<F, T: ?Sized>(callback: F) -> Callback<F, T>
where
    F: Fn(&T) -> bool,
{
    Callback::new(callback)
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

impl<T: ?Sized, F: Spec<T>> Spec<T> for Invert<T, F> {
    fn accept(&self, value: &T) -> bool {
        !self.func.accept(value)
    }
}

impl<T: ?Sized, F: Spec<T>> Invert<T, F> {
    pub fn new(func: F) -> Self {
        Self {
            func,
            _phantom: Default::default(),
        }
    }
}

/// Inverts the output of a spec
pub fn not<T: ?Sized, F: Spec<T>>(spec: F) -> Invert<T, F> {
    Invert::new(spec)
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
    F: FnOnce() -> Result<T, R> + UnwindSafe,
    R: From<Box<dyn Any + Send + 'static>>,
{
    match catch_unwind(func) {
        Ok(o) => o,
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
pub enum Work {
    UpToDate,
    Skipped,
    Failed,
    Success,
}

macro_rules! ok {
    ($e:expr) => {Result::Ok($e)};
    ($($e:expr),+ $(,)?) => { Result::Ok(($($e),*))}
}
pub(crate) use ok;

/// A type that can be shared between multiple values.
pub type Shared<T> = Arc<Mutex<T>>;

pub trait InstanceOf: Any {
    fn instance_of<T: ?Sized + Any>(&self) -> bool {
        TypeId::of::<Self>() == TypeId::of::<T>()
    }
}

impl<T: ?Sized + Any> InstanceOf for T {}

/// Perform a function then report how long it took
pub fn measure_time<R, F: FnOnce() -> R>(name: &str, level: log::Level, func: F) -> R {
    let instant = Instant::now();
    let out = func();
    log!(
        level,
        "{} completion time: {:.3} sec.",
        name,
        instant.elapsed().as_secs_f32()
    );
    out
}

/// A generic way of performing an action on some type
pub trait Action<T, R> {
    /// Executes some action on a value of type `T`
    fn execute(self, on: T) -> R;
}

impl<T, R, F> Action<T, R> for F
where
    F: FnOnce(T) -> R,
    R: Sized,
{
    fn execute(self, on: T) -> R {
        (self)(on)
    }
}

/// A semi shared allows for one instance of RW access, and many access of Read
#[derive(Debug)]
pub struct SemiShared<T: Send + Sync> {
    writer_out: Arc<AtomicBool>,
    inner: Arc<parking_lot::RwLock<T>>,
}

impl<T: Send + Sync> SemiShared<T> {
    /// Create a new semi shared value
    pub fn new(value: T) -> Self {
        Self {
            writer_out: Arc::new(AtomicBool::new(false)),
            inner: Arc::new(parking_lot::RwLock::new(value)),
        }
    }

    /// Get the writer to this semi shared. Only on semi shared can exist at a time
    pub fn writer(&self) -> Option<SemiSharedWriter<T>> {
        if self
            .writer_out
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
            .is_ok()
        {
            Some(SemiSharedWriter {
                writer_out: self.writer_out.clone(),
                inner: self.inner.clone(),
            })
        } else {
            None
        }
    }

    /// Gets a reader to this semi shared. Any number of semi shared can exist at a time
    pub fn reader(&self) -> SemiSharedReader<T> {
        SemiSharedReader {
            inner: self.inner.clone()
        }
    }
}
assert_impl_all!(SemiShared<i32>: Send, Sync);

/// Has only read access over the underlying value.
#[derive(Debug, Clone)]
pub struct SemiSharedReader<T: Send + Sync> {
    inner: Arc<parking_lot::RwLock<T>>,
}

impl<T: Send + Sync> SemiSharedReader<T> {

    /// Get read access to the underlying type
    pub fn read(&self) -> RwLockReadGuard<T> {
        self.inner.read()
    }
}

/// Has only read access over the underlying value.
#[derive(Debug)]
pub struct SemiSharedWriter<T: Send + Sync> {
    writer_out: Arc<AtomicBool>,
    inner: Arc<parking_lot::RwLock<T>>,
}

impl<T: Send + Sync> SemiSharedWriter<T> {
    /// Get read access to the underlying type
    pub fn read(&self) -> RwLockReadGuard<T> {
        self.inner.read()
    }
    /// Gets write access to the underlying type
    pub fn write(&mut self) -> RwLockWriteGuard<T> {
        self.inner.write()
    }
}


impl<T: Send + Sync> Drop for SemiSharedWriter<T> {
    fn drop(&mut self) {
        self.writer_out
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::Relaxed)
            .expect("should never happen");
    }
}
