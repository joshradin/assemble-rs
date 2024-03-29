use crate::dependencies::configurations::Configuration;
use crate::dependencies::RegistryContainer;
use crate::identifier::ProjectId;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct ConfigurationHandler {
    owner: ProjectId,
    registries: Arc<Mutex<RegistryContainer>>,
    configurations: HashMap<String, Configuration>,
}

unsafe impl Send for ConfigurationHandler {}
unsafe impl Sync for ConfigurationHandler {}

impl ConfigurationHandler {
    pub(crate) fn new(owner: ProjectId, registries: &Arc<Mutex<RegistryContainer>>) -> Self {
        Self {
            owner,
            registries: registries.clone(),
            configurations: Default::default(),
        }
    }

    /// Create a [`Configuration`](Configuration) with a given `name`.
    ///
    /// Returns `None` if configuration already exists.
    pub fn create<S: AsRef<str>>(&mut self, name: S) -> &mut Configuration {
        let name = name.as_ref();
        if self.configurations.contains_key(name) {
            panic!("configuration with name {:?} already exists", name);
        }

        self.configurations
            .insert(name.to_string(), Configuration::new(name, &self.registries));
        self.get_mut(name).unwrap()
    }

    /// Create a [`Configuration`](Configuration) with a given `name`.
    ///
    /// Returns `None` if configuration already exists.
    pub fn create_with<S, F>(&mut self, name: S, configure: F) -> &mut Configuration
    where
        S: AsRef<str>,
        F: FnOnce(&mut Configuration),
    {
        self.create(name.as_ref());
        let mutable = self.get_mut(name.as_ref()).unwrap();
        (configure)(mutable);
        self.get_mut(name.as_ref()).unwrap()
    }

    /// Get a reference configuration if it exists
    pub fn get<S: AsRef<str>>(&self, name: S) -> Option<&Configuration> {
        self.configurations.get(name.as_ref())
    }

    /// Get a mutable reference to a configuration if it exists
    pub fn get_mut<S: AsRef<str>>(&mut self, name: S) -> Option<&mut Configuration> {
        self.configurations.get_mut(name.as_ref())
    }

    /// Get the owner of this handler
    pub fn owner(&self) -> &ProjectId {
        &self.owner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::dependencies::RegistryContainer;
    use crate::file_collection::FileCollection;
    use crate::flow::output::SinglePathOutputTask;

    use crate::project::buildable::Buildable;

    use crate::task::create_task::CreateTask;

    use crate::task::initialize_task::InitializeTask;
    use crate::task::task_io::TaskIO;
    use crate::task::up_to_date::UpToDate;

    use crate::{BuildResult, Executable, Project, Task};
    use std::collections::HashSet;
    use std::fmt::Debug;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn file_only_configuration() {
        let registries = Arc::new(Mutex::new(RegistryContainer::new()));

        let mut dependency_container = ConfigurationHandler::new(ProjectId::default(), &registries);

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
        impl TaskIO for TestArtifactTask {}

        impl Task for TestArtifactTask {
            fn task_action(_task: &mut Executable<Self>, _project: &Project) -> BuildResult {
                todo!()
            }
        }

        impl SinglePathOutputTask for TestArtifactTask {
            fn get_path(_task: &Executable<Self>) -> PathBuf {
                PathBuf::from("test.txt")
            }
        }

        let registries = Arc::new(Mutex::new(RegistryContainer::new()));

        let mut dependency_container = ConfigurationHandler::new(ProjectId::default(), &registries);

        let project = Project::temp("temp");
        let task = project
            .register_task::<TestArtifactTask>("artifactExample")
            .unwrap();

        let deps = dependency_container.create_with("libs", |config| {
            config.add_dependency(task.clone());
        });

        let built_by = project.with(|p| deps.get_dependencies(p)).unwrap();
        assert_eq!(
            deps.resolved().unwrap().files(),
            HashSet::from_iter([PathBuf::from("test.txt")])
        );
        assert_eq!(built_by, HashSet::from_iter([task.id().clone()]))
    }
}
