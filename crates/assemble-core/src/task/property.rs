//! Property trait

use crate::fingerprint::{Fingerprint, FINGER_PRINT_SIZE};
use crate::task::PropertyError::IncorrectTypeForProperty;
use serde::de::DeserializeOwned;
use serde::{Deserializer, Serialize, Serializer};
use std::any::{type_name, Any, TypeId};
use std::borrow::Borrow;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};

/// Mark an object as a property
pub trait Property: Clone {}

impl<P: Clone> Property for P {}

pub trait Input<const N: usize = FINGER_PRINT_SIZE>: Fingerprint<N> {
    /// Get whether this input has changed since last run
    fn changed(&self) -> bool;
}

pub trait Output<const N: usize = FINGER_PRINT_SIZE>: Fingerprint<N> {
    /// Get whether this input is up to date
    fn up_to_date(&self) -> bool;
}

#[derive(Debug, thiserror::Error)]
pub enum PropertyError {
    #[error("Property is Missing: {0:?}")]
    PropertyMissing(String),
    #[error("Property {property:?} is not of type {requested_type:?}")]
    IncorrectTypeForProperty {
        property: String,
        requested_type: String,
    },
    #[error("{0:?} already set as an input")]
    InputAlreadySet(String),
    #[error("{0:?} already set as an output")]
    OutputAlreadySet(String),
}

#[derive(Default)]
pub struct TaskProperties {
    inputs: HashSet<String>,
    outputs: HashSet<String>,
    inner_map: HashMap<String, Box<dyn Any>>,
}

impl TaskProperties {
    pub fn get<T: 'static + Property, I>(&self, index: I) -> Result<&T, PropertyError>
    where
        I: AsRef<str>,
    {
        let index = index.as_ref();
        self.inner_map
            .get(index)
            .ok_or(PropertyError::PropertyMissing(index.to_string()))
            .and_then(|box_ref| {
                box_ref
                    .downcast_ref()
                    .ok_or_else(|| IncorrectTypeForProperty {
                        property: index.to_string(),
                        requested_type: type_name::<T>().to_string(),
                    })
            })
    }

    pub fn get_mut<T: 'static + Property, I>(&mut self, index: &I) -> Result<&mut T, PropertyError>
    where
        I: AsRef<str>,
    {
        let index = index.as_ref();
        self.inner_map
            .get_mut(index)
            .ok_or(PropertyError::PropertyMissing(index.to_string()))
            .and_then(|box_ref| {
                box_ref
                    .downcast_mut()
                    .ok_or_else(|| IncorrectTypeForProperty {
                        property: index.to_string(),
                        requested_type: type_name::<T>().to_string(),
                    })
            })
    }

    pub fn set<T: 'static + Property, I: AsRef<str>>(&mut self, index: I, value: T) {
        let index = index.as_ref().to_string();
        match self.inner_map.entry(index) {
            Entry::Occupied(mut occ) => {
                occ.insert(Box::new(value));
            }
            Entry::Vacant(mut vac) => {
                vac.insert(Box::new(value));
            }
        };
    }

    pub fn len(&self) -> usize {
        self.inner_map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner_map.is_empty()
    }

    pub fn contains<Q>(&self, key: &Q) -> bool
    where
        String: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.inner_map.contains_key(key)
    }

    pub fn set_input<T: 'static + Input + Property>(
        &mut self,
        name: String,
        value: T,
    ) -> Result<(), PropertyError> {
        if self.inputs.contains(&name) {
            return Err(PropertyError::InputAlreadySet(name));
        } else if self.outputs.contains(&name) {
            return Err(PropertyError::OutputAlreadySet(name));
        }
        self.set(&name, value);
        self.inputs.insert(name);
        Ok(())
    }

    pub fn set_output<T: 'static + Output + Property>(
        &mut self,
        name: String,
        value: T,
    ) -> Result<(), PropertyError> {
        if self.outputs.contains(&name) {
            return Err(PropertyError::OutputAlreadySet(name));
        } else if self.inputs.contains(&name) {
            return Err(PropertyError::InputAlreadySet(name));
        }
        self.set(&name, value);
        self.outputs.insert(name);
        Ok(())
    }

    pub fn with_type<T: 'static + Property>(&self) -> TaskTypedProperties<T> {
        TaskTypedProperties {
            entries: self
                .inner_map
                .iter()
                .filter_map(|(key, value)| {
                    if (**value).type_id() == TypeId::of::<T>() {
                        value.downcast_ref::<T>().map(|val| (key.clone(), val))
                    } else {
                        None
                    }
                })
                .collect(),
        }
    }
}

impl Index<&str> for TaskProperties {
    type Output = dyn Any;

    fn index(&self, index: &str) -> &Self::Output {
        &self.inner_map[index]
    }
}

pub struct TaskTypedProperties<'l, T: Property> {
    entries: HashMap<String, &'l T>,
}

impl<'l, T: Property> TaskTypedProperties<'l, T> {
    pub fn get<I>(&self, index: &I) -> Option<&'l T>
    where
        String: Borrow<I>,
        I: Hash + Eq,
    {
        match self.entries.get(index) {
            None => None,
            Some(out) => Some(*out),
        }
    }
}

impl<'l, I, T: Property> Index<I> for TaskTypedProperties<'l, T>
where
    I: AsRef<str>,
{
    type Output = &'l T;

    fn index(&self, index: I) -> &Self::Output {
        self.entries.get(index.as_ref()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::task::property::TaskProperties;

    #[test]
    fn set_and_get() {
        let mut properties = TaskProperties::default();
        properties.set("value1", "test".to_string());
        properties.set("value2", 17);

        assert_eq!(properties.with_type::<String>()["value1"], "test")
    }
}

pub trait FromProperties {
    fn from_properties(properties: &mut TaskProperties) -> Self;
}
