//! Mark a type as immutable.
//!
//! Any type that is immutable can not be mutated, even with ownership

use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Hash, Default)]
pub struct Immutable<T: ?Sized> {
    value: Arc<T>,
}

impl<T: ?Sized> Immutable<T> {
    /// Create a new immutable object from a value without a known size
    pub fn from_boxed(value: Box<T>) -> Self {
        let arc: Arc<T> = Arc::from(value);
        Self { value: arc }
    }

    /// Create a new immutable object from a value without a known size
    pub fn from_arc(value: Arc<T>) -> Self {
        Self { value }
    }
}

impl<T> Immutable<T> {
    /// Create a new immutable object from a value with known size
    pub fn new(value: T) -> Self {
        Self {
            value: Arc::new(value),
        }
    }
}

impl<T: ?Sized> Deref for Immutable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &*self.value
    }
}

impl<T> AsRef<T> for Immutable<T> {
    fn as_ref(&self) -> &T {
        self.value.as_ref()
    }
}

impl<T: ?Sized> Clone for Immutable<T> {
    fn clone(&self) -> Self {
        Immutable::from_arc(self.value.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::immutable::Immutable;

    #[test]
    fn use_immutables() {
        let i1 = Immutable::new(1);
        let i2 = Immutable::new(2);

        assert!(i1 < i2);
    }

    #[test]
    fn create_from_boxed() {
        let slice: Box<[i32]> = Box::new([1, 2, 3]);
        let immutable = Immutable::from_boxed(slice);
        assert_eq!(immutable[1], 2);
        assert_eq!(&immutable[..], &[1, 2, 3]);
    }
}
