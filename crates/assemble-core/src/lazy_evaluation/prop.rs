use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::fs::{File, OpenOptions};

use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::{Arc, PoisonError, RwLock};

use serde::ser::Error as SerdeError;
use serde::{Serialize, Serializer};

use crate::identifier::Id;
use crate::identifier::TaskId;
use crate::lazy_evaluation::anonymous::AnonymousProvider;

use crate::lazy_evaluation::Error::PropertyNotSet;
use crate::lazy_evaluation::{IntoProvider, Provider, Wrapper};
use crate::lazy_evaluation::{ProviderError, ProviderExt};
use crate::prelude::ProjectResult;
use crate::project::buildable::Buildable;

use crate::error::PayloadError;
use crate::{provider, Project};

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

impl<T: 'static + Send + Sync + Clone> Default for Prop<T> {
    fn default() -> Self {
        Self::new(Id::default())
    }
}

impl<T: 'static + Send + Sync + Clone + Debug> Debug for Prop<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            match self.fallible_get() {
                Ok(v) => {
                    write!(f, "{:#?}", v)
                }
                Err(_) => {
                    write!(f, "<no value>")
                }
            }
        } else {
            let id = self.id.clone();
            let read = self.inner.read().unwrap();

            match &*read {
                PropInner::Unset => {
                    write!(f, "Prop {{ id: {:?} }}>", self.id)
                }
                PropInner::Provided(inner) => f
                    .debug_struct("Prop")
                    .field("id", &id)
                    .field("value", &inner)
                    .finish(),
            }
        }
    }
}

impl<T: 'static + Send + Sync + Clone + Display> Display for Prop<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.fallible_get() {
            Ok(v) => {
                write!(f, "{}", v)
            }
            Err(_) => {
                write!(f, "<unset>")
            }
        }
    }
}

impl<T: 'static + Send + Sync + Clone + Debug> Buildable for Prop<T> {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        let inner = self.inner.read().map_err(PayloadError::new)?;
        match &*inner {
            PropInner::Unset => Ok(HashSet::new()),
            PropInner::Provided(p) => p.get_dependencies(project),
        }
    }
}

impl<T: 'static + Send + Sync + Clone + Debug> Provider<T> for Prop<T> {
    fn missing_message(&self) -> String {
        match &*self.inner.read().unwrap() {
            PropInner::Unset => {
                format!("{:?} has no value", self.id)
            }
            PropInner::Provided(p) => p.missing_message(),
        }
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

impl<P: AsRef<Path> + Send + Sync + Clone> Prop<P> {
    /// Attempt to open the file using given open_options
    pub fn open(&self, open_options: &OpenOptions) -> ProjectResult<File> {
        let path = self.fallible_get().map_err(PayloadError::new)?;
        Ok(open_options.open(path).map_err(PayloadError::new)?)
    }

    /// Attempt to read a file
    pub fn read(&self) -> ProjectResult<File> {
        self.open(OpenOptions::new().read(true))
    }

    /// Attempt to read a file
    pub fn create(&self) -> ProjectResult<File> {
        self.open(OpenOptions::new().write(true).create(true))
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
    Provided(Box<dyn Provider<T>>),
}

impl<T: Send + Sync + Clone> PropInner<T> {
    fn new() -> Self {
        Self::Unset
    }

    fn set<P: 'static + Provider<T>>(&mut self, value: P) -> Option<T> {
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

    fn take_inner(&mut self) -> Option<Box<dyn Provider<T>>> {
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
#[derive(Clone)]
pub struct VecProp<T: Send + Sync + Clone> {
    id: Id,
    prop: Arc<RwLock<Vec<AnonymousProvider<Vec<T>>>>>,
}

assert_impl_all!(VecProp<PathBuf>: Provider<Vec<PathBuf>>);
assert_impl_all!(VecProp<String>: Provider<Vec<String>>);

impl<T: 'static + Send + Sync + Clone> Default for VecProp<T> {
    fn default() -> Self {
        Self::new(Id::default())
    }
}

impl<T: 'static + Send + Sync + Clone> Buildable for VecProp<T> {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        self.prop
            .read()
            .map_err(PayloadError::new)?
            .iter()
            .map(|s| s.get_dependencies(project))
            .collect::<ProjectResult<Vec<_>>>()
            .map(|s| s.into_iter().flatten().collect())
    }
}

impl<T: 'static + Send + Sync + Clone> Debug for VecProp<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let inner = self.prop.read().map_err(|_| fmt::Error)?;
        write!(f, "VecProp ")?;
        let mut debug_vec = f.debug_list();
        for prov in &*inner {
            debug_vec.entry(prov);
        }
        debug_vec.finish()
    }
}

impl<T: 'static + Send + Sync + Clone> Provider<Vec<T>> for VecProp<T> {
    fn missing_message(&self) -> String {
        let read = self.prop.read().expect("poisoned");
        let first_missing = read
            .iter()
            .filter(|p| p.try_get().is_none())
            .map(|prop| prop.missing_message())
            .next();
        match first_missing {
            None => {
                format!("{} missing unknown value", self.id)
            }
            Some(msg) => format!("{} vector missing value > {}", self.id, msg),
        }
    }

    /// A vec prop will never return an empty vec
    fn try_get(&self) -> Option<Vec<T>> {
        let read = self.prop.read().expect("poisoned");
        read.iter()
            .map(|p| p.try_get())
            .collect::<Option<Vec<Vec<T>>>>()
            .map(|o| o.into_iter().flatten().collect())
    }

    fn fallible_get(&self) -> Result<Vec<T>, ProviderError> {
        let read = self.prop.read().expect("poisoned");
        read.iter()
            .map(|p| p.fallible_get())
            .collect::<Result<Vec<Vec<T>>, _>>()
            .map(|v| v.into_iter().flatten().collect())
    }
}

impl<T: 'static + Send + Sync + Clone> VecProp<T> {
    /// create a new vec prop with a given id
    pub fn new(id: Id) -> Self {
        Self {
            id,
            prop: Arc::new(RwLock::new(vec![])),
        }
    }

    /// Resets this property to contain only the values from the provider
    pub fn from<I: IntoIterator<Item = T> + Clone + Send + Sync, P: IntoProvider<I>>(
        &mut self,
        values: P,
    ) where
        P::Provider: 'static,
        I: 'static,
    {
        let mut write = self.prop.write().expect("poisoned");
        write.clear();
        let anonymous: AnonymousProvider<Vec<T>> =
            AnonymousProvider::new(values.into_provider().map(|v| v.into_iter().collect()));
        write.push(anonymous);
    }

    /// Push a value to the vector
    pub fn push_with<P>(&mut self, value: P)
    where
        P: IntoProvider<T>,
        P::Provider: 'static,
    {
        let mut write = self.prop.write().expect("vec panicked");
        let anonymous = AnonymousProvider::new(value.into_provider().map(|v| vec![v]));
        write.push(anonymous);
    }

    /// Push all value to the vector
    pub fn push_all_with<P, I>(&mut self, value: P)
    where
        I: IntoIterator<Item = T> + Clone + Send + Sync + 'static,
        P: IntoProvider<I>,
        P::Provider: 'static,
    {
        let mut write = self.prop.write().expect("vec panicked");
        let anonymous = AnonymousProvider::new(
            value
                .into_provider()
                .map(|v| v.into_iter().collect::<Vec<_>>()),
        );
        write.push(anonymous);
    }

    /// Push a value to the vector
    pub fn push<V>(&mut self, value: V)
    where
        V: Into<T>,
    {
        let value = value.into();
        self.push_with(provider!(move || value.clone()))
    }

    /// Push a value to the vector
    pub fn push_all<V, I: IntoIterator<Item = V>>(&mut self, value: I)
    where
        V: Into<T>,
    {
        for x in value {
            self.push(x);
        }
    }

    /// Clears the contents of the vector
    pub fn clear(&mut self) {
        let mut write = self.prop.write().expect("vec panicked");
        write.clear();
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

impl<T: Serialize> Serialize for VecProp<T>
where
    T: 'static + Send + Sync + Clone,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.fallible_get()
            .map_err(S::Error::custom)?
            .serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use crate::identifier::Id;
    use crate::lazy_evaluation::providers::Zip;
    use crate::lazy_evaluation::{AnyProp, Prop, Provider};
    use crate::lazy_evaluation::{ProviderExt, VecProp};
    use crate::provider;

    #[test]
    fn create_property() {
        let prop = AnyProp::new::<i32>("value".into());
        let mut ty_prop = prop.into_ty::<i32>().unwrap();
        let cloned = ty_prop.clone();
        ty_prop.set_with(provider!(|| 15)).unwrap();
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
        prop.push_with(provider!(|| 1));
        prop.extend([provider!(|| 2), provider!(|| 3)]);
        assert_eq!(other_prop.get(), vec![1, 2, 3]);
    }

    #[test]
    fn list_properties_join() {
        let mut prop = VecProp::<i32>::default();
        prop.from(Zip::new(
            provider!(|| vec![1, 2]),
            provider!(|| vec![3, 4]),
            |left, right| left.into_iter().chain(right).collect::<Vec<_>>(),
        ));
        assert_eq!(prop.get(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn vec_prop_missing() {
        let mut vec_prop = VecProp::<i32>::new(Id::from("test"));
        let mut prop1 = Prop::new(Id::from("prop1"));
        let mut prop2 = Prop::new(Id::from("prop2"));
        vec_prop.push_with(prop1.clone());
        vec_prop.push_with(prop2.clone());
        vec_prop.push_all_with(provider!(|| vec![1, 2]));
        assert_eq!(
            vec_prop.missing_message(),
            format!(":test vector missing value > {}", prop1.missing_message())
        );
        assert!(vec_prop.try_get().is_none());
        prop1.set(0).unwrap();
        assert_eq!(
            vec_prop.missing_message(),
            format!(":test vector missing value > {}", prop2.missing_message())
        );
        assert!(vec_prop.try_get().is_none());
        prop2.set(0).unwrap();
        assert_eq!(vec_prop.get(), vec![0, 0, 1, 2]);
    }
}
