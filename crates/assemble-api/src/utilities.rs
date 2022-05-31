use std::any::Any;
use std::fmt::Debug;
use std::ops::DerefMut;
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
