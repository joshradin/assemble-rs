//! Task Properties (2.0) does a better job at storing task information

use crate::__export::TaskId;
use crate::identifier::InvalidId;
use crate::project::buildable::{BuildByContainer, Buildable, BuiltBy, IntoBuildable};
use crate::project::ProjectError;
use crate::properties::{AnyProp, Prop};
use crate::Project;
use std::any::type_name;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut};

/// Stores Task Properties
#[derive(Default)]
pub struct TaskProperties {
    owner_task_id: TaskId,
    inputs: BuildByContainer,
    property_map: HashMap<String, AnyProp>,
}

impl TaskProperties {
    pub fn new(owner_task_id: TaskId) -> Self {
        Self {
            owner_task_id,
            inputs: BuildByContainer::default(),
            property_map: HashMap::new(),
        }
    }

    pub fn input<B: IntoBuildable>(&mut self, buildable: B)
    where
        <B as IntoBuildable>::Buildable: 'static,
    {
        self.inputs.add(buildable.into_buildable());
    }

    /// Gets a property as an output of this task.
    pub fn get_output<S: AsRef<str>>(&self, name: S) -> Option<BuiltBy<&AnyProp>> {
        self.property_map
            .get(name.as_ref())
            .map(|prop| BuiltBy::new(self.owner_task_id.clone(), prop))
    }

    /// Gets a property as an output of this task. Forces this object to be built by the owning task.
    pub fn get_output_ty<T: Clone + Send + Sync + 'static>(
        &self,
        name: &str,
    ) -> Option<BuiltBy<Prop<T>>> {
        self.get(name)
            .map(|prop| BuiltBy::new(self.owner_task_id.clone(), prop))
    }

    /// Shorthand for setting an input and adding it as a property.
    ///
    /// # Panic
    /// Will panic if property already exists. This is checked before the input is set.
    pub fn set_input_prop<S: AsRef<str>, B: Buildable + Clone + Send + Sync + 'static>(
        &mut self,
        name: S,
        value: B,
    ) {
        let name = name.as_ref();
        if self.property_map.contains_key(name) {
            panic!(
                "Attempting to add input property {}, but it already exists",
                name
            );
        }

        self.input(value.clone());
        let mut prop = self.owner_task_id.prop::<B>(name).unwrap();
        prop.set(value).expect("couldn't set value");
        self.property_map.insert(name.to_string(), prop.into());
    }

    pub fn insert<T: Clone + Send + Sync + 'static>(&mut self, key: &str, value: T) {
        let mut prop: Prop<T> = self.owner_task_id.prop(key).unwrap();
        prop.set(value).unwrap();
        self.property_map.insert(key.to_string(), prop.into());
    }

    pub fn get<T: Clone + Send + Sync + 'static>(&self, name: &str) -> Option<Prop<T>> {
        let prop = self.property_map.get(name)?;
        prop.as_ty().ok()
    }

    pub fn owner_task_id(&self) -> &TaskId {
        &self.owner_task_id
    }
}

impl Deref for TaskProperties {
    type Target = HashMap<String, AnyProp>;

    fn deref(&self) -> &Self::Target {
        &self.property_map
    }
}

impl DerefMut for TaskProperties {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.property_map
    }
}

impl Debug for TaskProperties {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct(type_name::<Self>());
        for (name, prop) in &self.property_map {
            debug_struct.field(&*name, prop);
        }
        debug_struct.finish()
    }
}

impl Buildable for TaskProperties {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        self.inputs.get_dependencies(project)
    }
}

#[cfg(test)]
mod tests {
    use crate::file_collection::FileCollection;
    use crate::properties::task_properties::TaskProperties;
    use crate::properties::{AnyProp, Provides};

    #[test]
    fn set_properties() {
        let mut props = TaskProperties::default();
        props.insert::<i32>("input", 13);

        let prop = props.get_output_ty::<i32>("input").unwrap();
        let value = prop.get();
        assert_eq!(value, 13);

        let files = FileCollection::default();

        props.set_input_prop("sources", files);
    }
}
