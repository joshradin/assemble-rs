use crate::defaults::plugins::BasePlugin;
use crate::defaults::tasks::Empty;
use crate::dependencies::dependency_container::ConfigurationHandler;
use crate::dependencies::project_dependency::ProjectUrlError;
use crate::dependencies::{AcquisitionError, RegistryContainer};
use crate::exception::BuildException;
use crate::file::RegularFile;
use crate::file_collection::FileSet;
use crate::flow::output::VariantHandler;
use crate::flow::shared::{Artifact, ConfigurableArtifact, ImmutableArtifact};
use crate::identifier::{is_valid_identifier, Id, InvalidId, ProjectId, TaskId, TaskIdFactory};
use crate::lazy_evaluation::{Prop, Provider, ProviderError};
use crate::logging::{LoggingControl, LOGGING_CONTROL};
use crate::plugins::extensions::{ExtensionAware, ExtensionContainer, ExtensionError};
use crate::plugins::{Plugin, PluginError};
use crate::resources::InvalidResourceLocation;
use crate::task::flags::{OptionsDecoderError, OptionsSlurperError};
use crate::task::task_container::{FindTask, TaskContainer};
use crate::task::{AnyTaskHandle, Executable};
use crate::task::{Task, TaskHandle};
use crate::workspace::{Dir, WorkspaceDirectory, WorkspaceError};
use crate::{lazy_evaluation, BuildResult, Workspace};
use log::debug;
use once_cell::sync::OnceCell;
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::Infallible;
use std::error::Error;
use std::fmt::{write, Debug, Display, Formatter};
use std::io;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Not};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::{
    Arc, Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError,
    Weak,
};
use tempfile::TempDir;

pub mod buildable;
pub mod configuration;
pub mod error;
pub mod requests;
pub mod subproject;
pub mod variant;

use crate::error::PayloadError;
pub use error::*;

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
pub struct Project {
    project_id: ProjectId,
    task_id_factory: TaskIdFactory,
    task_container: TaskContainer,
    workspace: Workspace,
    build_dir: Prop<PathBuf>,
    applied_plugins: Vec<String>,
    variants: VariantHandler,
    self_reference: OnceCell<Weak<RwLock<Project>>>,
    properties: HashMap<String, Option<String>>,
    default_tasks: Vec<TaskId>,
    registries: Arc<Mutex<RegistryContainer>>,
    configurations: ConfigurationHandler,
    subprojects: HashMap<ProjectId, SharedProject>,
    parent_project: OnceCell<SharedProject>,
    root_project: OnceCell<Weak<RwLock<Project>>>,
    extensions: ExtensionContainer,
}

impl Debug for Project {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Project {:?}", self.project_id)
    }
}

impl Display for Project {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
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
        let name = file
            .clone()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        Self::in_dir_with_id(file, ProjectId::new(&name)?)
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
        Self::in_dir_with_id_and_root(path, id, None)
    }

    /// Creates an assemble project in a specified directory.
    fn in_dir_with_id_and_root<Id: TryInto<ProjectId>, P: AsRef<Path>>(
        path: P,
        id: Id,
        root: Option<&SharedProject>,
    ) -> error::Result<SharedProject>
    where
        ProjectError: From<<Id as TryInto<ProjectId>>::Error>,
    {
        let id = id.try_into()?;
        LOGGING_CONTROL.in_project(id.clone());
        let factory = TaskIdFactory::new(id.clone());
        let mut build_dir = Prop::new(id.join("buildDir")?);
        build_dir.set(path.as_ref().join("build"))?;
        let registries = Arc::new(Mutex::new(Default::default()));
        let dependencies = ConfigurationHandler::new(id.clone(), &registries);
        let mut project = SharedProject::new(Self {
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
        });
        {
            let clone = project.clone();
            debug!("Initializing project task container...");
            project.with_mut(|proj| {
                proj.task_container.init(&clone);
                proj.self_reference.set(clone.weak()).unwrap();
                if let Some(root) = root {
                    proj.root_project.set(root.weak()).unwrap();
                } else {
                    proj.root_project.set(proj.as_shared().weak()).unwrap();
                }
                proj.apply_plugin::<BasePlugin>()
                    .expect("could not apply base plugin");
                Ok::<(), PayloadError<ProjectError>>(())
            })?;
        }
        LOGGING_CONTROL.reset();
        Ok(project)
    }

    /// Get the id of the project
    pub fn id(&self) -> &ProjectId {
        &self.project_id
    }

    pub fn build_dir(&self) -> impl Provider<PathBuf> + Clone {
        self.build_dir.clone()
    }

    /// Always set as relative to the project dir
    pub fn set_build_dir(&mut self, dir: &str) {
        let dir = self.workspace.dir(dir).unwrap();
        let path = dir.rel_path();
        self.build_dir.set(path).unwrap();
    }

    pub fn registered_tasks(&self) -> Vec<TaskId> {
        self.task_container
            .get_tasks()
            .into_iter()
            .cloned()
            .collect()
    }

    pub fn is_valid_representation(&self, repr: &str, task: &TaskId) -> bool {
        task.is_shorthand(repr) || task.this_id().is_shorthand(repr)
    }

    pub fn find_task_id(&self, repr: &str) -> error::Result<TaskId> {
        let mut output = Vec::new();
        for task_id in self.task_container.get_tasks() {
            if self.is_valid_representation(repr, &task_id) {
                output.push(task_id.clone());
            }
        }
        match &output[..] {
            [] => Err(ProjectError::NoIdentifiersFound(repr.to_string()).into()),
            [one] => Ok(one.clone()),
            _many => Err(ProjectError::TooManyIdentifiersFound(output, repr.to_string()).into()),
        }
    }

    /// Try to resolve a task id
    pub fn resolve_task_id(&self, id: &str) -> error::Result<TaskId> {
        let potential: Vec<TaskId> = self
            .task_container
            .get_tasks()
            .into_iter()
            .filter(|task_id| self.is_valid_representation(id, task_id))
            .cloned()
            .collect::<Vec<_>>();

        match &potential[..] {
            [] => Err(ProjectError::InvalidIdentifier(InvalidId(id.to_string())).into()),
            [once] => Ok((*once).clone()),
            alts => panic!("Many found for {}: {:?}", id, alts),
        }
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
        self.workspace.create_file(path).map_err(PayloadError::from)
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
        self.workspace.absolute_path().to_path_buf()
    }

    /// The project directory for the root directory
    pub fn root_dir(&self) -> PathBuf {
        self.root_project().with(|p| p.project_dir())
    }

    pub fn apply_plugin<P: Plugin>(&mut self) -> error::Result<()> {
        let plugin = P::default();
        plugin.apply(self).map_err(PayloadError::from)
    }

    /// Gets a list of all eligible tasks for a given string. Must return one task per project, but
    /// can return multiples tasks over multiple tasks.
    pub fn find_eligible_tasks(&self, task_id: &str) -> ProjectResult<Option<Vec<TaskId>>> {
        let mut output = vec![];
        match self.find_task_id(task_id) {
            Ok(task) => {
                output.push(task);
            }
            Err(e) => {
                let kind = e.kind();
                if !matches!(kind, ProjectError::NoIdentifiersFound(_)) {
                    return Err(e);
                }
            }
        }
        for subproject in self.subprojects() {
            if let Some(tasks) = subproject.find_eligible_tasks(task_id)? {
                output.extend(tasks);
            }
        }
        Ok(output.is_empty().not().then_some(output))
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

    pub fn as_shared(&self) -> SharedProject {
        SharedProject::try_from(self.self_reference.get().unwrap().clone()).unwrap()
    }

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
        let mut registries = self.registries.lock()?;
        configure(registries.deref_mut())
    }

    /// apply a function on the registries container
    pub fn registries<R, F: FnOnce(&RegistryContainer) -> ProjectResult<R>>(
        &self,
        configure: F,
    ) -> ProjectResult<R> {
        let registries = self.registries.lock()?;
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
        let id = ProjectId::from(self.project_id().join(project)?);
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
        let root_shared = self.root_project().clone();
        let self_shared = self.as_shared();
        let id = ProjectId::from(self.project_id.join(name)?);
        let shared = self.subprojects.entry(id.clone()).or_insert_with(|| {
            Project::in_dir_with_id_and_root(path, id.clone(), Some(&root_shared)).unwrap()
        });
        shared.with_mut(|p| p.parent_project.set(self_shared).unwrap());
        shared.with_mut(configure)
    }

    /// Gets the root project of this project.
    pub fn root_project(&self) -> SharedProject {
        SharedProject::try_from(self.root_project.get().unwrap().clone()).unwrap()
    }

    /// Gets the parent project of this project, if it exists
    pub fn parent_project(&self) -> Option<&SharedProject> {
        self.parent_project.get()
    }

    pub fn variants(&self) -> &VariantHandler {
        &self.variants
    }

    pub fn variants_mut(&mut self) -> &mut VariantHandler {
        &mut self.variants
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

#[derive(Debug, Clone)]
pub struct SharedProject(Arc<RwLock<Project>>);

pub type WeakSharedProject = Weak<RwLock<Project>>;

impl SharedProject {
    fn new(project: Project) -> Self {
        Self(Arc::new(RwLock::new(project)))
    }

    pub(crate) fn weak(&self) -> WeakSharedProject {
        Arc::downgrade(&self.0)
    }

    pub fn with<F, R>(&self, func: F) -> R
    where
        F: FnOnce(&Project) -> R,
    {
        let mut guard = self
            .0
            .try_read()
            .expect("Couldn't get read access to project");
        let project = &*guard;
        (func)(project)
    }

    pub fn with_mut<F, R>(&self, func: F) -> R
    where
        F: FnOnce(&mut Project) -> R,
    {
        let mut guard = self
            .0
            .try_write()
            .expect("Couldn't get write access to project");
        let project = &mut *guard;
        (func)(project)
    }

    pub fn guard<'g, T, F: Fn(&Project) -> &T + 'g>(&'g self, func: F) -> ProjectResult<Guard<T>> {
        let guard = match self.0.try_read() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(e)) => return Err(ProjectError::from(e).into()),
            Err(TryLockError::WouldBlock) => {
                panic!("Accessing this immutable guard would block")
            }
        };
        Ok(Guard::new(guard, func))
    }

    pub fn guard_mut<'g, T, F1, F2>(
        &'g self,
        ref_getter: F1,
        mut_getter: F2,
    ) -> ProjectResult<GuardMut<T>>
    where
        F1: Fn(&Project) -> &T + 'g,
        F2: Fn(&mut Project) -> &mut T + 'g,
    {
        let guard = match self.0.try_write() {
            Ok(guard) => guard,
            Err(TryLockError::Poisoned(e)) => return Err(ProjectError::from(e).into()),
            Err(TryLockError::WouldBlock) => {
                panic!("Accessing this guard would block")
            }
        };
        Ok(GuardMut::new(guard, ref_getter, mut_getter))
    }

    pub fn tasks(&self) -> GuardMut<TaskContainer> {
        self.guard_mut(
            |project| project.task_container(),
            |project| project.task_container_mut(),
        )
        .expect("couldn't safely get task container")
    }

    pub fn register_task<T: Task + Send + Debug + 'static>(
        &self,
        id: &str,
    ) -> ProjectResult<TaskHandle<T>> {
        self.tasks().register_task::<T>(id)
    }

    /// Find a task with a given name
    pub fn get_task<I>(&self, id: I) -> ProjectResult<AnyTaskHandle>
    where
        TaskContainer: FindTask<I>,
    {
        self.task_container().get_task(id)
    }

    /// Gets a typed task
    pub fn get_typed_task<T: Task + Send, I>(&self, id: I) -> ProjectResult<TaskHandle<T>>
    where
        TaskContainer: FindTask<I>,
    {
        self.task_container().get_task(id).and_then(|id| {
            id.as_type::<T>()
                .ok_or(ProjectError::custom("invalid task type").into())
        })
    }

    pub fn get_subproject<P>(&self, project: P) -> error::Result<SharedProject>
    where
        P: AsRef<str>,
    {
        self.with(|p| p.get_subproject(project).cloned())
    }

    /// Gets a list of all eligible tasks for a given string. Must return one task per project, but
    /// can return multiples tasks over multiple tasks.
    pub fn find_eligible_tasks(&self, task_id: &str) -> ProjectResult<Option<Vec<TaskId>>> {
        self.with(|p| p.find_eligible_tasks(task_id))
    }

    pub fn apply_plugin<P: Plugin>(&self) -> ProjectResult {
        self.with_mut(|project| project.apply_plugin::<P>())
    }

    pub fn task_container(&self) -> Guard<TaskContainer> {
        self.guard(|project| project.task_container())
            .expect("couldn't safely get task container")
    }

    pub(crate) fn task_id_factory(&self) -> Guard<TaskIdFactory> {
        self.guard(|project| project.task_id_factory())
            .expect("couldn't safely get task id factory")
    }

    /// Get access to the registries container
    pub fn registries<F, R>(&self, func: F) -> R
    where
        F: FnOnce(&mut RegistryContainer) -> R,
    {
        self.with(|r| {
            let mut registries_lock = r.registries.lock().unwrap();
            let registries = &mut *registries_lock;
            func(registries)
        })
    }

    /// Get a guard to the dependency container within the project
    pub fn configurations_mut(&mut self) -> GuardMut<ConfigurationHandler> {
        self.guard_mut(
            |project| project.configurations(),
            |project| project.configurations_mut(),
        )
        .expect("couldn't safely get dependencies container")
    }

    /// Get a guard to the dependency container within the project
    pub fn configurations(&self) -> Guard<ConfigurationHandler> {
        self.guard(|project| project.configurations())
            .expect("couldn't safely get dependencies container")
    }

    pub fn workspace(&self) -> Guard<Workspace> {
        self.guard(|p| &p.workspace)
            .expect("couldn't get workspace")
    }
}

impl Display for SharedProject {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.read().unwrap())
    }
}

impl Default for SharedProject {
    fn default() -> Self {
        Project::new().unwrap()
    }
}

impl TryFrom<Weak<RwLock<Project>>> for SharedProject {
    type Error = ();

    fn try_from(value: Weak<RwLock<Project>>) -> std::result::Result<Self, Self::Error> {
        Ok(SharedProject(value.upgrade().ok_or(())?))
    }
}

/// Provides a shortcut around the project
pub struct Guard<'g, T> {
    guard: RwLockReadGuard<'g, Project>,
    getter: Box<dyn Fn(&Project) -> &T + 'g>,
}

impl<'g, T> Guard<'g, T> {
    pub fn new<F>(guard: RwLockReadGuard<'g, Project>, getter: F) -> Self
    where
        F: Fn(&Project) -> &T + 'g,
    {
        Self {
            guard,
            getter: Box::new(getter),
        }
    }
}

impl<'g, T> Deref for Guard<'g, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let guard = &*self.guard;
        (self.getter)(guard)
    }
}

/// Provides a shortcut around the project
pub struct GuardMut<'g, T> {
    guard: RwLockWriteGuard<'g, Project>,
    ref_getter: Box<dyn Fn(&Project) -> &T + 'g>,
    mut_getter: Box<dyn Fn(&mut Project) -> &mut T + 'g>,
}

impl<'g, T> Deref for GuardMut<'g, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let guard = &*self.guard;
        (self.ref_getter)(guard)
    }
}

impl<'g, T> DerefMut for GuardMut<'g, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let guard = &mut *self.guard;
        (self.mut_getter)(guard)
    }
}

impl<'g, T> GuardMut<'g, T> {
    pub fn new<F1, F2>(guard: RwLockWriteGuard<'g, Project>, ref_getter: F1, mut_getter: F2) -> Self
    where
        F1: Fn(&Project) -> &T + 'g,
        F2: Fn(&mut Project) -> &mut T + 'g,
    {
        Self {
            guard,
            ref_getter: Box::new(ref_getter),
            mut_getter: Box::new(mut_getter),
        }
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
        self.parent_project().map(GetProjectId::project_id)
    }

    fn root_id(&self) -> ProjectId {
        self.root_project().project_id()
    }
}

impl GetProjectId for SharedProject {
    fn project_id(&self) -> ProjectId {
        self.with(|p| p.project_id())
    }

    fn parent_id(&self) -> Option<ProjectId> {
        self.with(GetProjectId::parent_id)
    }

    fn root_id(&self) -> ProjectId {
        self.with(GetProjectId::root_id)
    }
}

#[cfg(test)]
mod test {
    use crate::defaults::tasks::Empty;
    use crate::logging::{init_root_log, LoggingArgs};
    use crate::project::{Project, SharedProject};
    use crate::task::task_container::TaskContainer;
    use log::LevelFilter;
    use std::env;
    use std::path::PathBuf;
    use tempfile::{tempdir, TempDir};

    #[test]
    fn create_tasks() {
        let mut project = SharedProject::default();

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
        let project = project.0.read().unwrap();
        let file = project.file("test1").expect("Couldn't make file from &str");
        assert_eq!(file.path(), project.project_dir().join("test1"));
        let file = project
            .file("test2".to_string())
            .expect("Couldn't make file from String");
        assert_eq!(file.path(), project.project_dir().join("test2"));
    }
}
