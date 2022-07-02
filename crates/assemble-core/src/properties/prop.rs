use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::sync::{Arc, PoisonError, RwLock, TryLockError};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use crate::identifier::Id;
use crate::properties::Error::PropertyNotSet;
use crate::properties::{AsProvider, Provides, Wrapper};

assert_impl_all!(AnyProp: Send, Sync, Clone, Debug);

/// A property that contains a value. Once a property is created, only types that
/// use the correct type are used.
#[derive(Clone)]
pub struct AnyProp {
    id: Id,
    inner: Arc<dyn Any + Send + Sync>,
    ty_id: TypeId
}

impl AnyProp {

    /// Creates a new, empty property
    pub fn new<T : 'static + Send + Sync + Clone>(id: Id) -> Self {
        let ty_id = TypeId::of::<T>();
        Self {
            id: id.clone(),
            inner: Arc::new(Prop::<T>::new(id)),
            ty_id
        }
    }

    /// Creates a new property with this value set
    pub fn with<T : 'static + AsProvider<R>, R : 'static + Send + Sync + Clone>(id: Id, value: T) -> Self {
        let mut output = Self::new::<R>(id);
        output.set(value).unwrap();
        output
    }

    /// Sets
    pub fn set<T : 'static + AsProvider<R>, R: 'static + Send + Sync + Clone>(&mut self, value: T) -> Result<(), Error> {
        let mut with_ty: Prop<R> = self.clone().into_ty()?;
        with_ty.set_with(value)?;
        Ok(())
    }

    pub fn into_ty<T : 'static + Send + Sync + Clone>(self) -> Result<Prop<T>, Error> {
        Prop::try_from(self)
    }

    pub fn as_ty<T : 'static + Send + Sync + Clone>(&self) -> Result<Prop<T>, Error> {
        Prop::try_from(self.clone())
    }
}

impl Debug for AnyProp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Property {}", self.id)
    }
}

impl<T : Send +Sync + Clone> From<Prop<T>> for AnyProp {
    fn from(prop: Prop<T>) -> Self {
        let id = prop.id.clone();
        let ty_id = TypeId::of::<T>();
        AnyProp {
            id,
            inner: Arc::new(prop),
            ty_id
        }
    }
}





/// A typed prop
pub struct Prop<T : 'static + Send + Sync + Clone> {
    id: Id,
    ty_string: String,
    inner: Arc<RwLock<PropInner<T>>>,
}

impl<T: 'static + Send + Sync + Clone> Debug for Prop<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TyProp<type = {}> {}", self.ty_string, self.id)
    }
}

impl<T: 'static + Send + Sync + Clone> Provides<T> for Prop<T> {
    fn try_get(&self) -> Option<T> {
        self.fallible_get().ok()
    }
}

impl<T: 'static + Send + Sync + Clone> Prop<T> {

    pub fn new(id: Id) -> Self {
        Self {
            id,
            ty_string: std::any::type_name::<T>().to_string(),
            inner: Arc::new(RwLock::new(PropInner::Unset))
        }
    }


    pub fn with_value(value: T) -> Self {
        let mut output = Self::new(Default::default());
        output.set(value).unwrap();
        output
    }

    pub fn set_with<P : 'static + AsProvider<T>>(&mut self, val: P) -> Result<(), Error> {
        let mut inner = self.inner.write()?;
        let provider = val.as_provider();
        inner.set(provider);
        Ok(())
    }

    pub fn set<P>(&mut self, val: P) -> Result<(), Error>
        where T : From<P>
    {
        self.set_with(Wrapper(val.into()))
    }

    pub fn fallible_get(&self) -> Result<T, Error> {
        let inner = self.inner.read()?;
        match &*inner {
            PropInner::Unset => {
                Err(PropertyNotSet)
            }
            PropInner::Provided(provo) => {
                provo
                    .deref()
                    .try_get()
                    .ok_or(PropertyNotSet)
            }
        }
    }
}

impl<T : 'static + Send + Sync + Clone> TryFrom<AnyProp> for Prop<T> {
    type Error = Error;

    fn try_from(value: AnyProp) -> Result<Self, Self::Error> {
        let ty_id = TypeId::of::<T>();
        if value.ty_id != ty_id {
            Err(Error::TypeMismatch {
                expected: value.ty_id,
                found: ty_id
            })
        } else {
            let cloned = value.inner.clone();
            let downcast = cloned.downcast_ref::<Prop<T>>().unwrap();
            let take = downcast.clone();
            Ok(take)
        }
    }
}

impl<T : 'static + Send + Sync + Clone> Clone for Prop<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            ty_string: self.ty_string.clone(),
            inner: self.inner.clone()
        }
    }
}

enum PropInner<T : Send +Sync + Clone> {
    Unset,
    Provided(Box<dyn Provides<T>>)
}

impl<T : Send +Sync + Clone> PropInner<T> {
    fn new() -> Self {
        Self::Unset
    }


    fn set<P : 'static + Provides<T>>(&mut self, value: P) -> Option<T> {
        let get = self.get();
        *self = PropInner::Provided(Box::new(value));
        get
    }

    fn get(&self) -> Option<T> {
        match self {
            PropInner::Unset => { None }
            PropInner::Provided(provider) => {
                provider.as_ref()
                    .try_get()
            }
        }
    }
}


#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Property's lock was poisoned")]
    LockPoisonError,
    #[error("Type Mismatch (Expected = {:?}, Found = {:?})", expected, found)]
    TypeMismatch {
        expected: TypeId,
        found: TypeId
    },
    #[error("Property has no value set")]
    PropertyNotSet,
}

impl<T> From<PoisonError<T>> for Error {
    fn from(_: PoisonError<T>) -> Self {
        Self::LockPoisonError
    }
}


#[cfg(test)]
mod tests {
    use crate::identifier::Id;
    use crate::properties::{AnyProp, Provides};
    use crate::properties::ProvidesExt;

    #[test]
    fn create_property() {
        let mut prop = AnyProp::new::<i32>("value".into());
        let mut ty_prop = prop.into_ty::<i32>().unwrap();
        let cloned = ty_prop.clone();
        ty_prop.set_with(|| 15).unwrap();
        let gotten = ty_prop.get();
        assert_eq!(gotten, 15i32);

        let prop = ty_prop.map(|i| i*i);
        let get = prop.get();
        assert_eq!(get, 15i32*15);
        assert_eq!(cloned.get(), 15i32);
    }
}