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
    use std::fmt::{Debug, Formatter};
    use std::path::PathBuf;
    use tempfile::{TempDir, tempfile, tempfile_in};
    use super::*;
    use crate::dependencies::RegistryContainer;
    use crate::file_collection::FileCollection;
    use crate::flow::output::ArtifactTask;
    use crate::{BuildResult, Executable, Project, Task};
    use crate::__export::{CreateTask, InitializeTask, TaskId};
    use crate::flow::shared::ImmutableArtifact;
    use crate::project::buildable::{Buildable, IntoBuildable};
    use crate::project::ProjectResult;
    use crate::task::ExecutableTask;
    use crate::task::flags::{OptionDeclarations, OptionsDecoder};
    use crate::task::up_to_date::UpToDate;

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

    #[test]
    fn add_artifact_task() {
        #[derive(Debug, Default)]
        struct TestArtifactTask;

        impl UpToDate for TestArtifactTask {}
        impl InitializeTask for TestArtifactTask {}

        impl Task for TestArtifactTask {
            fn task_action(_task: &mut Executable<Self>, _project: &Project) -> BuildResult {
                todo!()
            }
        }

        impl ArtifactTask for TestArtifactTask {
            fn get_artifact(task: &Executable<Self>) -> ImmutableArtifact {
                ImmutableArtifact::new(PathBuf::from("test.txt"))
            }
        }

        let registries = Arc::new(Mutex::new(RegistryContainer::new()));

        let mut dependency_container = DependencyContainer::new(&registries);

        let project =Project::temp("temp");
        let task = project.register_task::<TestArtifactTask>("artifactExample").unwrap();

        let deps = dependency_container.create_with("libs", |config| {
            config.add_dependency(task.clone());
        });

        let resolved = deps.resolved().expect("couldn't resolve configuration");

        println!("resolved = {:#?}", resolved);

        let built_by = project.with(|p| resolved.get_dependencies(p)).unwrap();
        assert_eq!(resolved.files(), HashSet::from_iter([PathBuf::from("test.txt")]));
        assert_eq!(built_by, HashSet::from_iter([task.id().clone()]))
    }
}
