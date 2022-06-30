//! Task Properties (2.0) does a better job at storing task information

use crate::__export::TaskId;
use crate::identifier::InvalidId;
use crate::project::buildable::{Buildable, BuiltBy, BuiltByHandler, TaskDependency};
use crate::project::ProjectError;
use crate::properties::{Prop, TyProp};
use crate::Project;
use std::collections::{HashMap, HashSet};
use std::ops::{Deref, DerefMut};

/// Stores Task Properties
#[derive(Default)]
pub struct TaskProperties {
    owner_task_id: TaskId,
    inputs: BuiltByHandler,
    property_map: HashMap<String, Prop>,
}

impl TaskProperties {
    pub fn new(owner_task_id: TaskId) -> Self {
        Self {
            owner_task_id,
            inputs: BuiltByHandler::default(),
            property_map: HashMap::new(),
        }
    }

    pub fn input<B: Buildable>(&mut self, buildable: &B) {
        self.inputs.add(buildable);
    }

    /// Gets a property as an output of this task.
    pub fn get_output<S: AsRef<str>>(&self, name: S) -> Option<BuiltBy<&Prop>> {
        self.property_map.get(name.as_ref())
            .map(|prop| BuiltBy::new(self.owner_task_id.clone(), prop))
    }

    /// Gets a property as an output of this task. Forces this object to be built by the owning task.
    pub fn get_output_ty<T : Clone + Send + Sync + 'static>(&self, name: &str) -> Option<BuiltBy<TyProp<T>>> {
        self.get(name)
            .map(|prop| BuiltBy::new(self.owner_task_id.clone(), prop))
    }

    /// Shorthand for setting an input and adding it as a property.
    ///
    /// # Panic
    /// Will panic if property already exists. This is checked before the input is set.
    pub fn set_input_prop<S: AsRef<str>, B: Buildable + Clone + Send + Sync + 'static>(&mut self, name: S, value: B) {
        let name = name.as_ref();
        if self.property_map.contains_key(name) {
            panic!(
                "Attempting to add input property {}, but it already exists",
                name
            );
        }

        self.input(&value);
        let prop = self.owner_task_id.prop::<B>(name).unwrap();
        self.property_map.insert(name.to_string(), prop.into());
    }

    pub fn insert<T : Clone + Send + Sync + 'static>(&mut self, key: &str, value: T) {

        let mut prop = self.owner_task_id.prop(key).unwrap();
        prop.set(value).unwrap();
        self.property_map.insert(key.to_string(), prop.into());
    }

    pub fn get<T :  Clone + Send + Sync + 'static>(&self, name: &str) -> Option<TyProp<T>> {
        let prop = self.property_map.get(name)?;
        prop.as_ty()
            .ok()
    }

}

impl Deref for TaskProperties {
    type Target = HashMap<String, Prop>;

    fn deref(&self) -> &Self::Target {
        &self.property_map
    }
}

impl DerefMut for TaskProperties {

    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.property_map
    }
}

impl TaskDependency for TaskProperties {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        self.inputs.get_dependencies(project)
    }
}

#[cfg(test)]
mod tests {
    use crate::file_collection::FileCollection;
    use crate::properties::task_properties::TaskProperties;
    use crate::properties::{Prop, Provides};

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
