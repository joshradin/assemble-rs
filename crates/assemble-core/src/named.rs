use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};

/// A named value with a given type
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Named<T> {
    value: T,
    name: String,
}

impl<T> Debug for Named<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.name)
    }
}

impl<T> Named<T> {

    /// Create a new named object
    pub fn new(name: impl AsRef<str>, val: T) -> Self {
        Self {
            value: val,
            name: name.as_ref().to_string()
        }
    }

    /// Get the value of named object
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Get the name of the named object
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl<T> Deref for Named<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for Named<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

/// Turn a value into a named value
pub trait IntoNamed : Sized {

    /// Add a name to this object
    fn named<S : AsRef<str>>(self, name: S) -> Named<Self> {
        Named::new(name, self)
    }
}


impl <T> IntoNamed for T {

}
