use crate::defaults::plugins::BasePlugin;
use std::cell::RefCell;

use crate::dependencies::dependency_container::ConfigurationHandler;

use crate::dependencies::RegistryContainer;

use crate::file::RegularFile;

use crate::flow::output::VariantHandler;
use crate::flow::shared::ConfigurableArtifact;
use crate::identifier::{Id, InvalidId, ProjectId, TaskId, TaskIdFactory};
use crate::lazy_evaluation::{Prop, Provider};
use crate::logging::LOGGING_CONTROL;
use crate::plugins::extensions::{ExtensionAware, ExtensionContainer};
use crate::plugins::{Plugin, PluginAware, PluginManager};

use crate::task::task_container::TaskContainer;
use crate::task::AnyTaskHandle;
use crate::task::{Task, TaskHandle};
use crate::workspace::WorkspaceDirectory;
use crate::Workspace;
use log::debug;
use once_cell::sync::OnceCell;

use std::collections::HashMap;

use std::fmt::{Debug, Display, Formatter};

use std::ops::{Deref, DerefMut, Not};
use std::path::{Path, PathBuf};

use parking_lot::{ReentrantMutex, ReentrantMutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::sync::{Arc, Mutex, TryLockError, Weak};
use tempfile::TempDir;

pub mod buildable;
#[cfg(test)]
pub mod dev;
pub mod error;
pub mod finder;
pub mod requests;
pub mod shared;
pub mod variant;

use crate::error::PayloadError;
use crate::prelude::{Settings, SettingsAware};
use crate::project::finder::TaskPath;
pub use error::*;
use shared::{SharedProject, TrueSharableProject, WeakSharedProject};

pub mod prelude {
    pub use super::Project;
    pub use crate::dependencies::project_dependency::CreateProjectDependencies;
}

/// The Project contains the tasks, layout information, and other related objects that would help
/// with project building.
///
/// The project itself should be able to provide all information required to build a project, but
/// should not be the driver of the building itself. Instead, project visitors should be driven to
/// create project files.
///
/// By default, projects are created in the current directory.
///
/// # Example
/// ```
/// # use assemble_core::Project;
/// # use assemble_core::defaults::tasks::Empty;
/// # let mut project = Project::temp(None);
/// let mut task_provider = project.tasks().register_task::<Empty>("hello_world").expect("Couldn't create 'hello_task'");
/// task_provider.configure_with(|empty, _project| {
///     empty.do_first(|_, _| {
///         println!("Hello, World");
///         Ok(())
///     }).unwrap();
///     Ok(())
/// }).unwrap();
/// ```
#[derive(Debug)]
pub struct Project {
    settings: Option<Weak<RwLock<Settings>>>,
    project_id: ProjectId,
    task_id_factory: TaskIdFactory,
    task_container: TaskContainer,
    workspace: Workspace,
    build_dir: Prop<PathBuf>,
    applied_plugins: Vec<String>,
    variants: VariantHandler,
    self_reference: OnceCell<WeakSharedProject>,
    properties: HashMap<String, Option<String>>,
    default_tasks: Vec<TaskId>,
    registries: Arc<Mutex<RegistryContainer>>,
    configurations: ConfigurationHandler,
    subprojects: HashMap<ProjectId, SharedProject>,
    parent_project: OnceCell<WeakSharedProject>,
    root_project: OnceCell<WeakSharedProject>,
    extensions: ExtensionContainer,
    plugin_manager: PluginManager<Project>,

    is_root: bool,
}

impl Drop for Project {
    fn drop(&mut self) {
        warn!("dropping project {}", self.project_id);
    }
}
//
// impl Debug for Project {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(f, "Project {:?}", self.project_id)
//     }
// }

impl Display for Project {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Project {}", self.project_id)
    }
}

impl Project {
    #[doc(hidden)]
    pub fn temp<'a, I: Into<Option<&'a str>>>(id: I) -> SharedProject {
        Self::in_dir_with_id(TempDir::new().unwrap().path(), id.into().unwrap_or("root")).unwrap()
    }

    /// Create a new Project, with the current directory as the the directory to load
    pub fn new() -> error::Result<SharedProject> {
        let file = std::env::current_dir().unwrap();
        let name = file.file_name().unwrap().to_str().unwrap().to_string();
        Self::in_dir_with_id(file, ProjectId::new(&name).map_err(PayloadError::new)?)
    }

    /// Creates an assemble project in a specified directory.
    pub fn in_dir(path: impl AsRef<Path>) -> error::Result<SharedProject> {
        let path = path.as_ref();
        Self::in_dir_with_id(path, path)
    }

    /// Creates an assemble project in the current directory using an identifier
    pub fn with_id<I: TryInto<ProjectId>>(id: I) -> error::Result<SharedProject>
    where
        ProjectError: From<<I as TryInto<ProjectId>>::Error>,
    {
        Self::in_dir_with_id(std::env::current_dir().unwrap(), id)
    }

    /// Creates an assemble project in a specified directory.
    pub fn in_dir_with_id<Id: TryInto<ProjectId>, P: AsRef<Path>>(
        path: P,
        id: Id,
    ) -> error::Result<SharedProject>
    where
        ProjectError: From<<Id as TryInto<ProjectId>>::Error>,
    {
        Self::in_dir_with_id_and_root(path, id, None, None)
    }

    /// Creates an assemble project in a specified directory.
    pub fn in_dir_with_id_and_root<Id: TryInto<ProjectId>, P: AsRef<Path>>(
        path: P,
        id: Id,
        root: Option<&SharedProject>,
        settings: Option<Weak<RwLock<Settings>>>,
    ) -> error::Result<SharedProject>
    where
        ProjectError: From<<Id as TryInto<ProjectId>>::Error>,
    {
        let id = id.try_into().map_err(PayloadError::new)?;
        LOGGING_CONTROL.in_project(id.clone());
        let factory = TaskIdFactory::new(id.clone());
        let mut build_dir = Prop::new(id.join("buildDir").map_err(PayloadError::new)?);
        build_dir
            .set(path.as_ref().join("build"))
            .map_err(PayloadError::new)?;
        let registries = Arc::new(Mutex::new(Default::default()));
        let dependencies = ConfigurationHandler::new(id.clone(), &registries);

        let project = SharedProject::new_cyclic(|cycle| {
            let mut project = Self {
                settings: settings.clone(),
                project_id: id,
                task_id_factory: factory.clone(),
                task_container: TaskContainer::new(factory),
                workspace: Workspace::new(path),
                build_dir,
                applied_plugins: Default::default(),
                variants: VariantHandler::new(),
                self_reference: OnceCell::new(),
                properties: Default::default(),
                default_tasks: vec![],
                registries,
                configurations: dependencies,
                subprojects: Default::default(),
                parent_project: OnceCell::new(),
                root_project: OnceCell::new(),
                extensions: ExtensionContainer::default(),
                plugin_manager: PluginManager::default(),
                is_root: root.is_none(),
            };

            project.task_container.init(cycle);
            project.self_reference.set(cycle.clone()).unwrap();
            if let Some(root) = root {
                project.root_project.set(root.weak()).unwrap();
            } else {
                project.root_project.set(cycle.clone()).unwrap();
            }

            project
        });

        project.apply_plugin::<BasePlugin>()?;

        LOGGING_CONTROL.reset();
        Ok(project)
    }

    /// Get the id of the project
    pub fn id(&self) -> &ProjectId {
        &self.project_id
    }

    /// Gets the directory where created files should be stored
    pub fn build_dir(&self) -> impl Provider<PathBuf> + Clone {
        self.build_dir.clone()
    }

    /// Always set as relative to the project dir
    pub fn set_build_dir(&mut self, dir: &str) {
        let dir = self.workspace.dir(dir).unwrap();
        let path = dir.rel_path();
        self.build_dir.set(path).unwrap();
    }

    /// Gets a list of all tasks by [`TaskId`](TaskId)
    pub fn registered_tasks(&self) -> Vec<TaskId> {
        self.task_container
            .get_tasks()
            .into_iter()
            .cloned()
            .collect()
    }

    /// Create files using some valid types
    ///
    /// Allowed types:
    /// - &str
    /// - String
    /// - Path
    /// - Regular File
    pub fn file<T: AsRef<Path>>(&self, any_value: T) -> error::Result<RegularFile> {
        let path = any_value.as_ref();
        debug!("trying to create/get file {:?}", path);
        self.workspace.create_file(path).map_err(PayloadError::new)
    }

    /// Run a visitor on the project
    pub fn visitor<R, V: VisitProject<R>>(&self, visitor: &mut V) -> R {
        visitor.visit(self)
    }

    /// Run a mutable visitor on the project
    pub fn visitor_mut<R, V: VisitMutProject<R>>(&mut self, visitor: &mut V) -> R {
        visitor.visit_mut(self)
    }

    /// The directory of the project
    pub fn project_dir(&self) -> PathBuf {
        self.workspace.absolute_path()
    }

    /// The project directory for the root directory
    pub fn root_dir(&self) -> PathBuf {
        self.root_project().with(|p| p.project_dir())
    }

    /// Get access to the task container
    pub fn task_container(&self) -> &TaskContainer {
        &self.task_container
    }

    /// Get access to the task container
    pub fn task_container_mut(&mut self) -> &mut TaskContainer {
        &mut self.task_container
    }

    /// Get an outgoing variant
    pub fn variant(&self, variant: &str) -> Option<impl Provider<ConfigurableArtifact>> {
        self.variants.get_artifact(variant)
    }

    /// Gets the shared reference version of this project
    pub fn as_shared(&self) -> SharedProject {
        SharedProject::try_from(self.self_reference.get().unwrap().clone()).unwrap()
    }

    /// Gets the factory for generating task ids
    pub fn task_id_factory(&self) -> &TaskIdFactory {
        &self.task_id_factory
    }

    pub fn properties(&self) -> &HashMap<String, Option<String>> {
        &self.properties
    }

    pub fn set_property(&mut self, key: String, value: impl Into<Option<String>>) {
        self.properties.insert(key, value.into());
    }

    pub fn get_property(&self, key: &str) -> Option<&Option<String>> {
        self.properties.get(key)
    }

    pub fn has_property(&self, key: &str) -> bool {
        self.properties.contains_key(key)
    }

    /// Gets the subprojects for this project.
    pub fn subprojects(&self) -> Vec<&SharedProject> {
        self.subprojects.values().collect()
    }

    /// Gets the default tasks for this project.
    ///
    /// Default tasks are executed if no other tasks are provided.
    pub fn default_tasks(&self) -> &Vec<TaskId> {
        &self.default_tasks
    }

    /// Set the default tasks for this project.
    pub fn set_default_tasks<I: IntoIterator<Item = TaskId>>(&mut self, iter: I) {
        self.default_tasks = Vec::from_iter(iter);
    }

    /// apply a configuration function on the registries container
    pub fn registries_mut<F: FnOnce(&mut RegistryContainer) -> ProjectResult>(
        &mut self,
        configure: F,
    ) -> ProjectResult {
        let mut registries = self.registries.lock().map_err(PayloadError::new)?;
        configure(registries.deref_mut())
    }

    /// apply a function on the registries container
    pub fn registries<R, F: FnOnce(&RegistryContainer) -> ProjectResult<R>>(
        &self,
        configure: F,
    ) -> ProjectResult<R> {
        let registries = self.registries.lock().map_err(PayloadError::new)?;
        configure(registries.deref())
    }

    /// Get the dependencies for this project
    pub fn configurations(&self) -> &ConfigurationHandler {
        &self.configurations
    }

    /// Get a mutable reference to the dependencies container for this project
    pub fn configurations_mut(&mut self) -> &mut ConfigurationHandler {
        &mut self.configurations
    }

    pub fn get_subproject<P: AsRef<str>>(&self, project: P) -> error::Result<&SharedProject> {
        let id = ProjectId::from(self.project_id().join(project).map_err(PayloadError::new)?);
        self.subprojects
            .get(&id)
            .ok_or(ProjectError::NoIdentifiersFound(id.to_string()).into())
    }

    /// Create a sub project with a given name. The path used is the `$PROJECT_DIR/name`
    pub fn subproject<F>(&mut self, name: &str, configure: F) -> ProjectResult
    where
        F: FnOnce(&mut Project) -> ProjectResult,
    {
        let path = self.project_dir().join(name);
        self.subproject_in(name, path, configure)
    }

    /// Create a sub project with a given name at a path.
    pub fn subproject_in<P, F>(&mut self, name: &str, path: P, configure: F) -> ProjectResult
    where
        F: FnOnce(&mut Project) -> ProjectResult,
        P: AsRef<Path>,
    {
        let root_shared = self.root_project();
        let self_shared = self.as_shared();
        let id = ProjectId::from(self.project_id.join(name).map_err(PayloadError::new)?);
        let shared = self.subprojects.entry(id.clone()).or_insert_with(|| {
            Project::in_dir_with_id_and_root(
                path,
                id.clone(),
                Some(&root_shared),
                self.settings.clone(),
            )
            .unwrap()
        });
        shared.with_mut(|p| p.parent_project.set(self_shared.weak()).unwrap());
        shared.with_mut(configure)
    }

    /// Gets the root project of this project.
    pub fn root_project(&self) -> SharedProject {
        SharedProject::try_from(self.root_project.get().unwrap().clone()).unwrap()
    }

    /// Gets the parent project of this project, if it exists
    pub fn parent_project(&self) -> Option<SharedProject> {
        self.parent_project
            .get()
            .and_then(|p| p.clone().upgrade().ok())
            .map(|s| s.into())
    }

    /// The variants of this project
    pub fn variants(&self) -> &VariantHandler {
        &self.variants
    }

    /// The mutable variants of this project
    pub fn variants_mut(&mut self) -> &mut VariantHandler {
        &mut self.variants
    }

    /// Gets the reference to the settings object
    fn settings(&self) -> Arc<RwLock<Settings>> {
        self.settings
            .as_ref()
            .and_then(|weak| weak.upgrade())
            .expect("settings not set")
    }
}

impl PluginAware for Project {
    fn plugin_manager(&self) -> &PluginManager<Self> {
        &self.plugin_manager
    }

    fn plugin_manager_mut(&mut self) -> &mut PluginManager<Self> {
        &mut self.plugin_manager
    }
}

impl ExtensionAware for Project {
    fn extensions(&self) -> &ExtensionContainer {
        &self.extensions
    }

    fn extensions_mut(&mut self) -> &mut ExtensionContainer {
        &mut self.extensions
    }
}

///  trait for visiting projects
pub trait VisitProject<R = ()> {
    /// Visit the project
    fn visit(&mut self, project: &Project) -> R;
}

/// trait for visiting project thats mutable
pub trait VisitMutProject<R = ()> {
    /// Visit a mutable project.
    fn visit_mut(&mut self, project: &mut Project) -> R;
}

impl SettingsAware for Project {
    fn with_settings<F: FnOnce(&Settings) -> R, R>(&self, func: F) -> R {
        self.settings().with_settings(func)
    }

    fn with_settings_mut<F: FnOnce(&mut Settings) -> R, R>(&mut self, func: F) -> R {
        self.settings().with_settings_mut(func)
    }
}

pub trait GetProjectId {
    fn project_id(&self) -> ProjectId;
    fn parent_id(&self) -> Option<ProjectId>;
    fn root_id(&self) -> ProjectId;

    /// Get whether this project is a root
    fn is_root(&self) -> bool {
        self.root_id() == self.project_id()
    }
}

impl GetProjectId for Project {
    fn project_id(&self) -> ProjectId {
        self.id().clone()
    }

    fn parent_id(&self) -> Option<ProjectId> {
        self.parent_project().as_ref().map(GetProjectId::project_id)
    }

    fn root_id(&self) -> ProjectId {
        self.root_project().project_id()
    }

    fn is_root(&self) -> bool {
        self.is_root
    }
}

#[cfg(test)]
mod test {
    use crate::defaults::tasks::Empty;
    use crate::logging::init_root_log;
    use crate::project::Project;

    use crate::project::shared::SharedProject;
    use crate::workspace::WorkspaceDirectory;
    use log::LevelFilter;
    use std::env;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn create_tasks() {
        let project = SharedProject::default();

        let mut provider = project.tasks().register_task::<Empty>("arbitrary").unwrap();
        provider.configure_with(|_, _| Ok(())).unwrap();
    }

    #[test]
    fn project_name_based_on_directory() {
        let path = PathBuf::from("parent_dir/ProjectName");
        let project = Project::in_dir(path).unwrap();

        assert_eq!(
            project.with(|p| p.id().clone()).to_string(),
            ":parent_dir:ProjectName"
        );
    }

    #[test]
    fn create_files_in_project() {
        init_root_log(LevelFilter::Debug, None);
        let temp_dir = TempDir::new_in(env::current_dir().unwrap()).unwrap();
        assert!(temp_dir.path().exists());
        let project = Project::temp(None);
        project.with(|project| {
            let file = project.file("test1").expect("Couldn't make file from &str");
            assert_eq!(file.path(), project.project_dir().join("test1"));
            let file = project
                .file("test2")
                .expect("Couldn't make file from String");
            assert_eq!(file.path(), project.project_dir().join("test2"));
        })
    }
}
