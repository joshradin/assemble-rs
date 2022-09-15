use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Arc, PoisonError, RwLock, TryLockError};

use serde::{Deserialize, Serialize};

use crate::identifier::Id;
use crate::identifier::TaskId;
use crate::prelude::ProjectResult;
use crate::project::buildable::Buildable;
use crate::project::error::ProjectError;
use crate::properties::providers::Map;
use crate::properties::Error::PropertyNotSet;
use crate::properties::ProvidesExt;
use crate::properties::{IntoProvider, Provides, Wrapper};
use crate::Project;

assert_impl_all!(AnyProp: Send, Sync, Clone, Debug);

/// A property that contains a value. Once a property is created, only types that
/// use the correct type are used.
#[derive(Clone)]
pub struct AnyProp {
    id: Id,
    inner: Arc<dyn Any + Send + Sync>,
    ty_id: TypeId,
}

impl AnyProp {
    /// Creates a new, empty property
    pub fn new<T: 'static + Send + Sync + Clone>(id: Id) -> Self {
        let ty_id = TypeId::of::<T>();
        Self {
            id: id.clone(),
            inner: Arc::new(Prop::<T>::new(id)),
            ty_id,
        }
    }

    /// Creates a new property with this value set
    pub fn with<T: 'static + IntoProvider<R>, R: 'static + Send + Sync + Clone>(
        id: Id,
        value: T,
    ) -> Self {
        let mut output = Self::new::<R>(id);
        output.set(value).unwrap();
        output
    }

    /// Sets
    pub fn set<T: 'static + IntoProvider<R>, R: 'static + Send + Sync + Clone>(
        &mut self,
        value: T,
    ) -> Result<(), Error> {
        let mut with_ty: Prop<R> = self.clone().into_ty()?;
        with_ty.set_with(value)?;
        Ok(())
    }

    pub fn into_ty<T: 'static + Send + Sync + Clone>(self) -> Result<Prop<T>, Error> {
        Prop::try_from(self)
    }

    pub fn as_ty<T: 'static + Send + Sync + Clone>(&self) -> Result<Prop<T>, Error> {
        Prop::try_from(self.clone())
    }
}

impl Debug for AnyProp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Property {}", self.id)
    }
}

impl<T: Send + Sync + Clone> From<Prop<T>> for AnyProp {
    fn from(prop: Prop<T>) -> Self {
        let id = prop.id.clone();
        let ty_id = TypeId::of::<T>();
        AnyProp {
            id,
            inner: Arc::new(prop),
            ty_id,
        }
    }
}

/// A typed prop
pub struct Prop<T: 'static + Send + Sync + Clone> {
    id: Id,
    ty_string: String,
    inner: Arc<RwLock<PropInner<T>>>,
}

impl<T: 'static + Send + Sync + Clone + Buildable> Buildable for Prop<T> {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        let prop = self.try_get().ok_or(Error::PropertyNotSet)?;
        prop.get_dependencies(project)
    }
}

impl<T: 'static + Send + Sync + Clone> Default for Prop<T> {
    fn default() -> Self {
        Self::new(Id::default())
    }
}

impl<T: 'static + Send + Sync + Clone + Debug> Debug for Prop<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(v) = self.try_get() {
            f.debug_struct(&format!("Prop<{}>", self.ty_string))
                .field("id", &self.id)
                .field("value", &v)
                .finish()
        } else {
            f.debug_struct(&format!("Prop<{}>", self.ty_string))
                .field("id", &self.id)
                .field("value", &Option::<()>::None)
                .field("missing reason", &self.missing_message())
                .finish()
        }
    }
}

impl<T: 'static + Send + Sync + Clone> Provides<T> for Prop<T> {
    fn missing_message(&self) -> String {
        format!("{:?} has no value", self.id)
    }

    fn try_get(&self) -> Option<T> {
        Self::fallible_get(self).ok()
    }
}

impl<T: 'static + Send + Sync + Clone> Prop<T> {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            ty_string: std::any::type_name::<T>().to_string(),
            inner: Arc::new(RwLock::new(PropInner::Unset)),
        }
    }

    pub fn with_name<S: AsRef<str>>(id: S) -> Self {
        Self {
            id: Id::new(id).unwrap(),
            ty_string: std::any::type_name::<T>().to_string(),
            inner: Arc::new(RwLock::new(PropInner::Unset)),
        }
    }

    pub fn with_value(value: T) -> Self {
        let mut output = Self::new(Default::default());
        output.set(value).unwrap();
        output
    }

    pub fn set_with<P: IntoProvider<T>>(&mut self, val: P) -> Result<(), Error>
    where
        <P as IntoProvider<T>>::Provider: 'static,
    {
        use crate::properties::ProvidesExt;
        let mut inner = self.inner.write()?;
        let provider = val.into_provider();
        inner.set(provider);
        Ok(())
    }

    pub fn set<P>(&mut self, val: P) -> Result<(), Error>
    where
        P: Into<T>,
    {
        self.set_with(Wrapper(val.into()))
    }

    fn fallible_get(&self) -> Result<T, Error> {
        let inner = self.inner.read()?;
        match &*inner {
            PropInner::Unset => Err(PropertyNotSet),
            PropInner::Provided(provo) => provo.deref().try_get().ok_or(PropertyNotSet),
        }
    }

    /// The identifier of the property
    pub fn id(&self) -> &Id {
        &self.id
    }
}

impl<T: 'static + Send + Sync + Clone> TryFrom<AnyProp> for Prop<T> {
    type Error = Error;

    fn try_from(value: AnyProp) -> Result<Self, Self::Error> {
        let ty_id = TypeId::of::<T>();
        if value.ty_id != ty_id {
            Err(Error::TypeMismatch {
                expected: value.ty_id,
                found: ty_id,
            })
        } else {
            let cloned = value.inner.clone();
            let downcast = cloned.downcast_ref::<Prop<T>>().unwrap();
            let take = downcast.clone();
            Ok(take)
        }
    }
}

impl<T: 'static + Send + Sync + Clone> Clone for Prop<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            ty_string: self.ty_string.clone(),
            inner: self.inner.clone(),
        }
    }
}

enum PropInner<T: Send + Sync + Clone> {
    Unset,
    Provided(Box<dyn Provides<T>>),
}

impl<T: Send + Sync + Clone> PropInner<T> {
    fn new() -> Self {
        Self::Unset
    }

    fn set<P: 'static + Provides<T>>(&mut self, value: P) -> Option<T> {
        let get = self.get();
        *self = PropInner::Provided(Box::new(value));
        get
    }

    fn get(&self) -> Option<T> {
        match self {
            PropInner::Unset => None,
            PropInner::Provided(provider) => provider.as_ref().try_get(),
        }
    }

    fn take_inner(&mut self) -> Option<Box<dyn Provides<T>>> {
        match std::mem::replace(self, Self::Unset) {
            PropInner::Unset => None,
            PropInner::Provided(p) => Some(p),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Property's lock was poisoned")]
    LockPoisonError,
    #[error("Type Mismatch (Expected = {:?}, Found = {:?})", expected, found)]
    TypeMismatch { expected: TypeId, found: TypeId },
    #[error("Property has no value set")]
    PropertyNotSet,
}

impl<T> From<PoisonError<T>> for Error {
    fn from(_: PoisonError<T>) -> Self {
        Self::LockPoisonError
    }
}

/// A vec prop is a special property that uses a list
#[derive(Clone, Debug)]
pub struct VecProp<T: 'static + Send + Sync + Clone> {
    prop: Prop<Vec<T>>,
}

impl<T: 'static + Send + Sync + Clone> Default for VecProp<T> {
    fn default() -> Self {
        Self::new(Id::default())
    }
}

impl<T: 'static + Send + Sync + Clone> Provides<Vec<T>> for VecProp<T> {
    /// A vec prop will never return an empty vec
    fn try_get(&self) -> Option<Vec<T>> {
        let mut output = vec![];
        if let Some(inner) = self.prop.try_get() {
            output.extend(inner);
        }
        Some(output)
    }
}

impl<T: 'static + Send + Sync + Clone> VecProp<T> {
    /// create a new vec prop with a given id
    pub fn new(id: Id) -> Self {
        let mut prop = Prop::new(id);
        prop.set(vec![]).unwrap();
        Self { prop }
    }

    /// Resets this property to contain only the values from the provider
    pub fn from<I: IntoIterator<Item = T> + Clone + Send + Sync, P: IntoProvider<I>>(
        &mut self,
        values: P,
    ) where
        P::Provider: 'static,
        I: 'static,
    {
        let provider = values.into_provider();
        let vector: Map<I, _, _, _> = provider.map(|v| v.into_iter().collect());
        self.prop.set_with(vector).unwrap();
    }

    /// Push a value to the vector
    pub fn push_with<P>(&mut self, value: P)
    where
        P: IntoProvider<T>,
        P::Provider: 'static,
    {
        let provider = {
            let mut inner = self.prop.inner.write().unwrap();
            inner.take_inner()
        };
        match provider {
            None => {
                self.prop
                    .set_with(value.into_provider().map(|v| vec![v]))
                    .unwrap();
            }
            Some(s) => {
                let value_p = value.into_provider();
                let provider = move || -> Option<Vec<T>> {
                    let mut original = s.try_get()?;
                    original.push(value_p.try_get()?);
                    Some(original)
                };
                self.prop.set_with(provider).unwrap();
            }
        }
    }

    /// Push a value to the vector
    pub fn push(&mut self, value: T) {
        self.push_with(move || value.clone())
    }
}

impl<P, T: 'static + Send + Sync + Clone> Extend<P> for VecProp<T>
where
    P: IntoProvider<T>,
    P::Provider: 'static,
{
    fn extend<I: IntoIterator<Item = P>>(&mut self, iter: I) {
        for p in iter {
            self.push_with(p);
        }
    }
}

impl<T: 'static + Send + Sync + Clone> IntoIterator for VecProp<T> {
    type Item = T;
    type IntoIter = <Vec<T> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.get().into_iter()
    }
}

#[cfg(test)]
mod tests {
    use crate::identifier::Id;
    use crate::properties::providers::Zip;
    use crate::properties::{AnyProp, Provides};
    use crate::properties::{ProvidesExt, VecProp};
    use itertools::Itertools;

    #[test]
    fn create_property() {
        let mut prop = AnyProp::new::<i32>("value".into());
        let mut ty_prop = prop.into_ty::<i32>().unwrap();
        let cloned = ty_prop.clone();
        ty_prop.set_with(|| 15).unwrap();
        let gotten = ty_prop.get();
        assert_eq!(gotten, 15i32);

        let prop = ty_prop.map(|i| i * i);
        let get = prop.get();
        assert_eq!(get, 15i32 * 15);
        assert_eq!(cloned.get(), 15i32);
    }

    #[test]
    fn list_properties_from() {
        let mut prop = VecProp::<i32>::default();
        let other_prop = prop.clone();
        prop.push_with(|| 1);
        prop.extend([|| 2, || 3]);
        assert_eq!(other_prop.get(), vec![1, 2, 3]);
    }

    #[test]
    fn list_properties_join() {
        let mut prop = VecProp::<i32>::default();
        prop.from(Zip::new(
            || vec![1, 2],
            || vec![3, 4],
            |mut left, right| left.into_iter().chain(right).collect::<Vec<_>>(),
        ));
        assert_eq!(prop.get(), vec![1, 2, 3, 4]);
    }
}
