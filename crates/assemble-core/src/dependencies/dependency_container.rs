use crate::dependencies::configurations::Configuration;
use crate::dependencies::RegistryContainer;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct DependencyContainer {
    registries: Arc<Mutex<RegistryContainer>>,
    configurations: HashMap<String, Configuration>,
}

impl DependencyContainer {
    pub(crate) fn new(registries: &Arc<Mutex<RegistryContainer>>) -> Self {
        Self {
            registries: registries.clone(),
            configurations: Default::default(),
        }
    }

    /// Create a [`Configuration`](Configuration) with a given `name`.
    ///
    /// Returns `None` if configuration already exists.
    pub fn create<S: AsRef<str>>(&mut self, name: S) -> &Configuration {
        let name = name.as_ref();
        if self.configurations.contains_key(name) {
            panic!("configuration with name {:?} already exists", name);
        }

        self.configurations
            .insert(name.to_string(), Configuration::new(name, &self.registries));
        self.get(name).unwrap()
    }

    /// Create a [`Configuration`](Configuration) with a given `name`.
    ///
    /// Returns `None` if configuration already exists.
    pub fn create_with<S, F>(&mut self, name: S, configure: F) -> &Configuration
    where
        S: AsRef<str>,
        F: FnOnce(&mut Configuration),
    {
        self.create(name.as_ref());
        let mutable = self.get_mut(name.as_ref()).unwrap();
        (configure)(mutable);
        self.get(name.as_ref()).unwrap()
    }

    /// Get a reference configuration if it exists
    pub fn get<S: AsRef<str>>(&self, name: S) -> Option<&Configuration> {
        self.configurations.get(name.as_ref())
    }

    /// Get a mutable reference to a configuration if it exists
    pub fn get_mut<S: AsRef<str>>(&mut self, name: S) -> Option<&mut Configuration> {
        self.configurations.get_mut(name.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use tempfile::{TempDir, tempfile, tempfile_in};
    use super::*;
    use crate::dependencies::RegistryContainer;
    use crate::file_collection::FileCollection;

    #[test]
    fn file_only_configuration() {
        let registries = Arc::new(Mutex::new(RegistryContainer::new()));

        let mut dependency_container = DependencyContainer::new(&registries);

        let temp_dir = TempDir::new().unwrap();

        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        let deps = dependency_container.create_with("libs", |config| {
            config.add_dependency(file1.clone());
            config.add_dependency(file2.clone());
        });

        let resolved = deps.resolved().expect("couldn't resolve configuration");

        println!("resolved = {:#}", resolved);

        assert_eq!(resolved.files(), HashSet::from_iter([file1, file2]));
    }
}
