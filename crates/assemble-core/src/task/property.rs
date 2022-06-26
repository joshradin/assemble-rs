//! Property trait

use crate::fingerprint::{Fingerprint, FINGER_PRINT_SIZE};
use crate::task::PropertyError::{IncorrectTypeForProperty, PropertyNotSet};
use serde::de::DeserializeOwned;
use serde::{Deserializer, Serialize, Serializer};
use std::any::{type_name, Any, TypeId};
use std::borrow::Borrow;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::sync::{Arc, RwLock};
use crate::exception::BuildException;
use crate::identifier::TaskId;
use crate::{Executable, Project};
use crate::project::buildable::{Buildable, BuiltBy, TaskDependenciesSet, TaskDependency};
use crate::project::ProjectError;

/// Mark an object as a property
pub trait Property: Clone+ Send + Sync {}

impl<P: Clone+ Send + Sync> Property for P {}

pub trait Input<const N: usize = FINGER_PRINT_SIZE>: Fingerprint<N> {
    /// Get whether this input has changed since last run
    fn changed(&self) -> bool;
}

pub trait Output<const N: usize = FINGER_PRINT_SIZE>: Fingerprint<N> + Debug {
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
    #[error("{0:?} not an output")]
    PropertyNotOutput(String),
    #[error("{0:?} not an input")]
    PropertyNotInput(String),
    #[error("Property not set, but trying to get in delayed")]
    PropertyNotSet
}

#[derive(Default)]
pub struct TaskProperties {
    parent_id: TaskId,
    built_bys: Vec<Arc<dyn Buildable>>,
    inputs: HashSet<String>,
    outputs: HashSet<String>,
    inner_map: HashMap<String, Box<dyn Any + Send + Sync>>,
}



impl TaskProperties {
    pub fn new(id: &TaskId) -> Self {
        Self {
            parent_id: id.clone(),
            ..Self::default()
        }
    }

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

    /// Get a specific output of a task. Unlike normal properties, this property can be considered built by this task.
    pub fn get_output<T: 'static + Property + Debug, I>(&self, index: I) -> Result<BuiltBy<&Arc<RwLock<Option<T>>>>, PropertyError>
        where
            I: AsRef<str>,
    {
        let index = index.as_ref();
        self.get(index)
            .map(|o| BuiltBy::new(self.parent_id.clone(), o))
            .and_then(|result| {
                if !self.outputs.contains(index) {
                    return Err(PropertyError::PropertyNotOutput(index.to_string()))
                } else {
                    Ok(result)
                }
            })
    }

    /// Get a specific output of a task. Unlike normal properties, this property can be considered built by this task. Doesn't check if the
    /// output exists.
    pub fn get_or_create_output<T: 'static + Property + Debug, I>(&mut self, index: I) -> Result<BuiltBy<&Arc<RwLock<Option<T>>>>, PropertyError>
        where
            I: AsRef<str>,
    {
        let index = index.as_ref();
        if !self.inner_map.contains_key(index) {
            self.set(index, Arc::new(RwLock::new(Option::<T>::None)));
        }
            self.get(index)
                .map(|o| BuiltBy::new(self.parent_id.clone(), o))

    }

    /// Get a specific input of a task. All inputs are specified to be "built" by something.
    pub fn get_input<T: 'static + Property + Debug, I>(&self, index: I) -> Result<&BuiltBy<T>, PropertyError>
        where
            I: AsRef<str>,
    {
        let index = index.as_ref();
        self.get::<BuiltBy<T>, _>(index)
            .and_then(|result| {
                if !self.inputs.contains(index) {
                    return Err(PropertyError::PropertyNotInput(index.to_string()))
                } else {
                    Ok(result)
                }
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

    pub fn set_input<T: 'static + Input + Property + Debug>(
        &mut self,
        name: String,
        value: impl Into<BuiltBy<T>>,
    ) -> Result<(), PropertyError> {
        if self.inputs.contains(&name) {
            return Err(PropertyError::InputAlreadySet(name));
        } else if self.outputs.contains(&name) {
            return Err(PropertyError::OutputAlreadySet(name));
        }
        let built_by = value.into();
        let built_by_arc = built_by.built_by();
        self.built_bys.push(built_by_arc);
        self.set(&name, built_by);
        self.inputs.insert(name);
        Ok(())
    }

    pub fn set_output<T: 'static + Output + Property>(
        &mut self,
        name: String,
        value: T,
    ) -> Result<(), PropertyError> {
        if self.inputs.contains(&name) {
            return Err(PropertyError::InputAlreadySet(name));
        }

        if self.inner_map.contains_key(&name) {
            let result = self.get_output(&name)?;
            let mut lock = result.write().unwrap();
            *lock = Some(value);
        } else {
            self.set(&name, Arc::new(RwLock::new(Some(value))));
        }


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

    pub fn extend(&mut self, properties: TaskProperties) {
        self.built_bys.extend(properties.built_bys);
        self.inputs.extend(properties.inputs);
        self.outputs.extend(properties.outputs);
        self.inner_map
            .extend(properties.inner_map)
    }

}

impl Index<&str> for TaskProperties {
    type Output = dyn Any;

    fn index(&self, index: &str) -> &Self::Output {
        &self.inner_map[index]
    }
}

impl Debug for TaskProperties {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskProperties")
            .field("task", &self.parent_id)
            .finish()
    }
}

impl Buildable for TaskProperties {
    fn get_build_dependencies(&self) -> Box<dyn TaskDependency> {
        let mut deps = TaskDependenciesSet::default();
        for built_by in &self.built_bys {
            deps.push(built_by.get_build_dependencies());
        }
        Box::new(deps)
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

pub trait FromProperties {
    fn from_properties(properties: &mut TaskProperties) -> Self;
}

#[derive(Clone)]
pub struct Delayed<T : Property + Debug> {
    task_id: TaskId,
    property_name: String,
    _ty: PhantomData<T>
}

impl<T : Property + Debug> Delayed<T> {

    pub(crate) fn new(task_id: TaskId, property_name: String) -> Self {
        Self { task_id, property_name, _ty: PhantomData }
    }
    pub fn resolve(self, project: &Project) -> Result<T, ProjectError> where T : 'static {
        // let task_ref = project.task_container().resolved_ref_task(self.task_id, project)?;
        // let task = task_ref.as_ref();
        // let result = task.properties().get_output::<T, _>(self.property_name)?;
        // let read = result.read().unwrap();
        // read.clone()
        //     .ok_or(PropertyError::PropertyNotSet).map_err(ProjectError::from)
        todo!()
    }
}

impl<T: Property + Send + Sync + Debug> Debug for Delayed<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Task {} delayed {}", self.task_id, self.property_name)
    }
}

impl<T : Property + Send + Sync + Debug> Buildable for Delayed<T> {
    fn get_build_dependencies(&self) -> Box<dyn TaskDependency> {
        self.task_id.get_build_dependencies()
    }
}



#[cfg(test)]
mod tests {
    use crate::task::property::TaskProperties;

    #[test]
    fn set_and_get() {
        let mut properties = TaskProperties::default();
        properties.set("value1", "tests".to_string());
        properties.set("value2", 17);

        assert_eq!(properties.with_type::<String>()["value1"], "tests")
    }
}

