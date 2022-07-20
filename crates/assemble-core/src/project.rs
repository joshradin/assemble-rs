use crate::defaults::plugins::BasePlugin;
use crate::defaults::tasks::Empty;
use crate::dependencies::Source;
use crate::exception::BuildException;
use crate::file::RegularFile;
use crate::file_collection::FileCollection;
use crate::flow::output::ArtifactHandler;
use crate::flow::shared::{Artifact, ConfigurableArtifact};
use crate::identifier::{is_valid_identifier, Id, InvalidId, ProjectId, TaskId, TaskIdFactory};
use crate::plugins::{Plugin, PluginError};
use crate::properties::{Prop, Provides};
use crate::task::task_container::{FindTask, TaskContainer};
use crate::task::{AnyTaskHandle, Executable};
use crate::task::{Task, TaskHandle};
use crate::workspace::{Dir, WorkspaceDirectory, WorkspaceError};
use crate::{properties, BuildResult, Workspace};
use log::debug;
use once_cell::sync::OnceCell;
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt::{write, Debug, Display, Formatter};
use std::io;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError,
};
use tempfile::TempDir;
use crate::task::flags::{OptionsDecoderError, OptionsSlurperError};

pub mod buildable;
pub mod configuration;
pub mod variant;
pub mod requests;

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
    variants: ArtifactHandler,
    self_reference: OnceCell<SharedProject>,
    properties: HashMap<String, Option<String>>
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
    pub fn new() -> Result<SharedProject> {
        Self::in_dir(std::env::current_dir().unwrap())
    }

    /// Creates an assemble project in a specified directory.
    pub fn in_dir(path: impl AsRef<Path>) -> Result<SharedProject> {
        let path = path.as_ref();
        Self::in_dir_with_id(path, path)
    }

    /// Creates an assemble project in the current directory using an identifier
    pub fn with_id<I: TryInto<ProjectId>>(id: I) -> Result<SharedProject>
    where
        ProjectError: From<<I as TryInto<ProjectId>>::Error>,
    {
        Self::in_dir_with_id(std::env::current_dir().unwrap(), id)
    }

    /// Creates an assemble project in a specified directory.
    pub fn in_dir_with_id<Id: TryInto<ProjectId>, P: AsRef<Path>>(
        path: P,
        id: Id,
    ) -> Result<SharedProject>
    where
        ProjectError: From<<Id as TryInto<ProjectId>>::Error>,
    {
        let id = id.try_into()?;
        let factory = TaskIdFactory::new(id.clone());
        let mut build_dir = Prop::new(id.join("buildDir")?);
        build_dir.set(path.as_ref().join("build"))?;
        let mut project = SharedProject(Arc::new(RwLock::new(Self {
            project_id: id,
            task_id_factory: factory.clone(),
            task_container: TaskContainer::new(factory),
            workspace: Workspace::new(path),
            build_dir,
            applied_plugins: Default::default(),
            variants: ArtifactHandler::new(),
            self_reference: OnceCell::new(),
            properties: Default::default()
        })));
        {
            let clone = project.clone();
            debug!("Initializing project task container...");
            project.with_mut(|proj| {
                proj.task_container.init(&clone);
                proj.self_reference.set(clone).unwrap();
                proj.apply_plugin::<BasePlugin>()
                    .expect("could not apply base plugin");
                Ok(())
            })?;
        }
        Ok(project)
    }

    /// Get the id of the project
    pub fn id(&self) -> &ProjectId {
        &self.project_id
    }

    pub fn build_dir(&self) -> impl Provides<PathBuf> + Clone {
        self.build_dir.clone()
    }

    /// Always set as relative to the project dir
    pub fn set_build_dir(&mut self, dir: &str) {
        let dir = self.workspace.dir(dir).unwrap();
        let path = dir.path();
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

    pub fn find_task_id(&self, repr: &str) -> Result<TaskId> {
        let mut output = Vec::new();
        for task_id in self.task_container.get_tasks() {
            if self.is_valid_representation(repr, &task_id) {
                output.push(task_id.clone());
            }
        }
        match &output[..] {
            [] => Err(ProjectError::NoIdentifiersFound(repr.to_string())),
            [one] => Ok(one.clone()),
            _many => Err(ProjectError::TooManyIdentifiersFound(
                output,
                repr.to_string(),
            )),
        }
    }

    /// Try to resolve a task id
    pub fn resolve_task_id(&self, id: &str) -> Result<TaskId> {
        let potential = self
            .task_container
            .get_tasks()
            .into_iter()
            .filter(|task_id| self.is_valid_representation(id, task_id))
            .collect::<Vec<_>>();

        match &potential[..] {
            [] => Err(ProjectError::InvalidIdentifier(InvalidId(id.to_string()))),
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
    pub fn file<T: AsRef<Path>>(&self, any_value: T) -> Result<RegularFile> {
        let path = any_value.as_ref();
        debug!("trying to create/get file {:?}", path);
        self.workspace.create_file(path).map_err(ProjectError::from)
    }

    pub fn sources(&self) -> impl IntoIterator<Item = &dyn Source> {
        vec![]
    }

    pub fn add_source<S: 'static + Source>(&mut self, source: S) {
        unimplemented!()
    }

    pub fn visitor<R, V: VisitProject<R>>(&self, visitor: &mut V) -> R {
        visitor.visit(self)
    }

    pub fn visitor_mut<R, V: VisitMutProject<R>>(&mut self, visitor: &mut V) -> R {
        visitor.visit_mut(self)
    }

    /// The directory of the project
    pub fn project_dir(&self) -> PathBuf {
        self.workspace.absolute_path().to_path_buf()
    }

    /// The project directory for the root directory
    pub fn root_dir(&self) -> PathBuf {
        self.project_dir()
    }

    pub fn apply_plugin<P: Plugin>(&mut self) -> Result<()> {
        let plugin = P::default();
        plugin.apply(self).map_err(ProjectError::from)
    }

    /// Get access to the task container
    pub fn task_container(&self) -> &TaskContainer {
        &self.task_container
    }

    /// Get access to the task container
    pub fn task_container_mut(&mut self) -> &mut TaskContainer {
        &mut self.task_container
    }

    pub fn variant(&self, variant: &str) -> Option<Arc<dyn Artifact>> {
        self.variants.get_artifact(variant)
    }

    pub fn as_shared(&self) -> SharedProject {
        self.self_reference.get().unwrap().clone()
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
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("No task identifier could be found for {0:?}")]
    NoIdentifiersFound(String),
    #[error("Too many task identifiers found for {1}. Found {0:?}")]
    TooManyIdentifiersFound(Vec<TaskId>, String),
    #[error("Identifier Missing: {0}")]
    IdentifierMissing(TaskId),
    #[error(transparent)]
    InvalidIdentifier(#[from] InvalidId),
    #[error(transparent)]
    PluginError(#[from] PluginError),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error("Inner Error {{ ... }}")]
    SomeError {},
    #[error("Infallible error occurred")]
    Infallible(#[from] Infallible),
    #[error(transparent)]
    PropertyError(#[from] properties::Error),
    #[error(transparent)]
    WorkspaceError(#[from] WorkspaceError),
    #[error("Invalid Type for file: {0}")]
    InvalidFileType(String),
    #[error("RwLock poisoned")]
    PoisonError,
    #[error("Actions already queried")]
    ActionsAlreadyQueried,
    #[error("No shared project was set")]
    NoSharedProjectSet,
    #[error(transparent)]
    OptionsDecoderError(#[from] OptionsDecoderError),
    #[error(transparent)]
    OptionsSlurperError(#[from] OptionsSlurperError),
}

impl<G> From<PoisonError<G>> for ProjectError {
    fn from(_: PoisonError<G>) -> Self {
        Self::PoisonError
    }
}

impl ProjectError {
    pub fn invalid_file_type<T>() -> Self {
        Self::InvalidFileType(std::any::type_name::<T>().to_string())
    }
}

impl From<Box<dyn Any + Send>> for ProjectError {
    fn from(e: Box<dyn Any + Send>) -> Self {
        Self::SomeError {}
    }
}

type Result<T> = std::result::Result<T, ProjectError>;
pub type ProjectResult<T = ()> = Result<T>;

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
pub struct SharedProject(pub(crate) Arc<RwLock<Project>>);

impl SharedProject {
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
            Err(TryLockError::Poisoned(e)) => return Err(ProjectError::from(e)),
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
            Err(TryLockError::Poisoned(e)) => return Err(ProjectError::from(e)),
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

    pub fn get_task<I>(&self, id: I) -> ProjectResult<AnyTaskHandle>
    where
        TaskContainer: FindTask<I>,
    {
        self.task_container().get_task(id)
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

        assert_eq!(project.with(|p| p.id().clone()), "ProjectName");
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
