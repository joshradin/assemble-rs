use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::sync::{Arc, PoisonError, RwLock, TryLockError};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};
use std::ops::Deref;
use crate::identifier::Id;
use crate::properties::Error::PropertyNotSet;
use crate::properties::{AsProvider, Provides, Wrapper};
use crate::task::PropertyError;

assert_impl_all!(Prop: Send, Sync, Clone, Debug);

/// A property that contains a value. Once a property is created, only types that
/// use the correct type are used.
#[derive(Clone)]
pub struct Prop {
    id: Id,
    inner: Arc<dyn Any + Send + Sync>,
    ty_id: TypeId
}

impl Prop {

    /// Creates a new, empty property
    pub fn new<T : 'static + Send + Sync + Clone>(id: Id) -> Self {
        let ty_id = TypeId::of::<T>();
        Self {
            id: id.clone(),
            inner: Arc::new(TyProp::<T>::new(id)),
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
        let mut with_ty: TyProp<R> = self.clone().with_ty()?;
        with_ty.set_with(value)?;
        Ok(())
    }

    pub fn with_ty<T : 'static + Send + Sync + Clone>(self) -> Result<TyProp<T>, Error> {
        TyProp::try_from(self)
    }
}

impl Debug for Prop {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Property {}", self.id)
    }
}

impl<T : Send +Sync + Clone> From<TyProp<T>> for Prop {
    fn from(prop: TyProp<T>) -> Self {
        let id = prop.id.clone();
        let ty_id = TypeId::of::<T>();
        Prop {
            id,
            inner: Arc::new(prop),
            ty_id
        }
    }
}





/// A typed prop
pub struct TyProp<T : 'static + Send + Sync + Clone> {
    id: Id,
    inner: Arc<RwLock<PropInner<T>>>,
}

impl<T: 'static + Send + Sync + Clone> Provides<T> for TyProp<T> {
    fn try_get(&self) -> Option<T> {
        self.fallible_get().ok()
    }
}

impl<T: 'static + Send + Sync + Clone> TyProp<T> {

    pub fn new(id: Id) -> Self {
        Self {
            id,
            inner: Arc::new(RwLock::new(PropInner::Unset))
        }
    }

    #[cfg(test)]
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

    pub fn set(&mut self, val: T) -> Result<(), Error> {
        self.set_with(Wrapper(val))
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

impl<T : 'static + Send + Sync + Clone> TryFrom<Prop> for TyProp<T> {
    type Error = Error;

    fn try_from(value: Prop) -> Result<Self, Self::Error> {
        let ty_id = TypeId::of::<T>();
        if value.ty_id != ty_id {
            Err(Error::TypeMismatch {
                expected: value.ty_id,
                found: ty_id
            })
        } else {
            let cloned = value.inner.clone();
            let downcast = cloned.downcast_ref::<TyProp<T>>().unwrap();
            let take = downcast.clone();
            Ok(take)
        }
    }
}

impl<T : 'static + Send + Sync + Clone> Clone for TyProp<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
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
    use crate::properties::{Prop, Provides};
    use crate::properties::ProvidesExt;

    #[test]
    fn create_property() {
        let mut prop = Prop::new::<i32>("value".into());
        let mut ty_prop = prop.with_ty::<i32>().unwrap();
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