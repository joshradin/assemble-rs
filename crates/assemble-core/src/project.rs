use std::any::Any;
use std::convert::Infallible;
use std::fmt::{Debug, Display, Formatter, write};
use std::io;
use crate::defaults::task::DefaultTask;
use crate::dependencies::Source;
use crate::task::task_container::{TaskContainer, TaskProvider};
use crate::task::{Empty, Executable, Task};
use crate::workspace::{Dir, WorkspaceDirectory, WorkspaceError};
use crate::{BuildResult, properties, Workspace};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use crate::exception::BuildException;
use crate::file::RegularFile;
use crate::file_collection::FileCollection;
use crate::identifier::{InvalidId, is_valid_identifier, ProjectId, TaskId, TaskIdFactory};
use crate::plugins::{Plugin, PluginError, ToPlugin};
use crate::properties::{Provides, Prop};

pub mod configuration;
pub mod buildable;

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
/// # use assemble_core::task::Empty;
/// let mut project = Project::default();
/// let mut task_provider = project.task::<Empty>("hello_world").expect("Couldn't create 'hello_task'");
/// task_provider.configure_with(|_empty, opts, project| {
///     opts.do_first(|_, _| {
///         println!("Hello, World");
///         Ok(())
///     });
///     Ok(())
/// });
/// ```
pub struct Project {
    project_id: ProjectId,
    task_id_factory: TaskIdFactory,
    task_container: TaskContainer<DefaultTask>,
    workspace: Workspace,
    build_dir: Prop<PathBuf>,
    applied_plugins: Vec<String>
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

impl Default for Project {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

impl Project {
    /// Create a new Project, with the current directory as the the directory to load
    pub fn new() -> Result<Self> {
        Self::in_dir(std::env::current_dir().unwrap())
    }

    /// Creates an assemble project in a specified directory.
    pub fn in_dir(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        Self::in_dir_with_id(path, path)
    }

    /// Creates an assemble project in the current directory using an identifier
    pub fn with_id<I : TryInto<ProjectId>>(id: I) -> Result<Self>
        where
            ProjectError : From<<I as TryInto<ProjectId>>::Error>{
        Self::in_dir_with_id(std::env::current_dir().unwrap(), id)
    }

    /// Creates an assemble project in a specified directory.
    pub fn in_dir_with_id<Id: TryInto<ProjectId>, P: AsRef<Path>>(path: P, id: Id) -> Result<Self>
    where
        ProjectError : From<<Id as TryInto<ProjectId>>::Error>
    {
        let id = id.try_into()?;
        let factory = TaskIdFactory::new(id.clone());
        let mut build_dir = Prop::new(id.join("buildDir")?);
        build_dir.set(path.as_ref().join("build"))?;
        Ok(Self {
            project_id: id,
            task_id_factory: factory,
            task_container: TaskContainer::new(),
            workspace: Workspace::new(path),
            build_dir,
            applied_plugins: Default::default(),
        })
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

    /// Creates a task within the project.
    ///
    /// When creating a task, the type of the task must be specified.
    ///
    /// # Error
    ///
    /// Tasks must be registered with unique identifiers, and will throw an error if task with this
    /// identifier already exists in this project. Tasks with identical names are allowed in sub-projects
    /// and sibling projects.
    pub fn task<T: 'static + Task<ExecutableTask = DefaultTask>>(
        &mut self,
        id: &str,
    ) -> Result<TaskProvider<T>> {
        let id = self.task_id_factory.create(id)?;
        Ok(self.task_container.register_task(id))
    }

    pub fn registered_tasks(&self) -> Vec<TaskId> {
        self.task_container.get_tasks()
    }

    pub fn is_valid_representation(&self, repr: &str, task: &TaskId) -> bool {
        task.is_shorthand(repr) || task.this_id().is_shorthand(repr)
    }

    pub fn find_task_id(&self, repr: &str) -> Result<TaskId> {
        let mut output = Vec::new();
        for task_id in self.task_container.get_tasks() {
            if self.is_valid_representation(repr, &task_id) {
                output.push(task_id);
            }
        }
        match &output[..] {
            [] => {
                Err(ProjectError::NoIdentifiersFound(repr.to_string()))
            }
            [one] => {
                Ok(one.clone())
            }
            _many => {
                Err(ProjectError::TooManyIdentifiersFound(output, repr.to_string()))
            }
        }
    }

    /// Try to resolve a task id
    pub fn resolve_task_id(&self, id: &str) -> Result<TaskId> {
        let potential = self.task_container.get_tasks()
            .into_iter()
            .filter(|task_id| self.is_valid_representation(id, task_id))
            .collect::<Vec<_>>();

        match &potential[..] {
            [] => Err(ProjectError::InvalidIdentifier(InvalidId(id.to_string()))),
            [once] => Ok(once.clone()),
            alts => panic!("Many found for {}: {:?}", id, alts)
        }
    }

    /// Create files using some valid types
    ///
    /// Allowed types:
    /// - &str
    /// - String
    /// - Path
    /// - Regular File
    pub fn file<T : AsRef<Path>>(&self, any_value: T) -> Result<RegularFile> {
        let path = any_value.as_ref();
        println!("trying to create/get file {:?}", path);
        self.workspace
            .create_file(path)
            .map_err(ProjectError::from)
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
    pub fn root_dir(&self) -> &Path {
        unimplemented!()
    }

    pub fn apply_plugin<P : Plugin>(&mut self, plugin: P) -> Result<()> {
        plugin.apply(self).map_err(ProjectError::from)
    }

    pub fn plugin<P : ToPlugin>(&self, p: P) -> Result<P::Plugin> {
        p.to_plugin(self).map_err(ProjectError::from)
    }

    /// Get access to the task container
    pub fn task_container(&self) -> TaskContainer<DefaultTask> {
        self.task_container.clone()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("No task identifier could be found for {0}")]
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
    #[error("Inner Error {:?} {{ ... }}", inner.type_id())]
    SomeError { inner: Box<dyn Any + Send> },
    #[error("Infallible error occurred")]
    Infallible(#[from] Infallible),
    #[error(transparent)]
    PropertyError(#[from] properties::Error),
    #[error(transparent)]
    WorkspaceError(#[from] WorkspaceError),
    #[error("Invalid Type for file: {0}")]
    InvalidFileType(String)
}

impl ProjectError {
    pub fn invalid_file_type<T>() -> Self {
        Self::InvalidFileType(std::any::type_name::<T>().to_string())
    }
}

impl From<Box<dyn Any + Send>> for ProjectError {
    fn from(e: Box<dyn Any + Send>) -> Self {
        Self::SomeError { inner: e }
    }
}

type Result<T> = std::result::Result<T, ProjectError>;

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



#[cfg(test)]
mod test {
    use std::env;
    use std::path::PathBuf;
    use tempfile::{TempDir, tempdir};
    use crate::DefaultTask;
    use crate::project::Project;
    use crate::task::Empty;
    use crate::task::task_container::TaskContainer;

    #[test]
    fn create_tasks() {
        let mut project = Project::default();

        let mut provider = project.task::<Empty>("tasks").unwrap();
        provider.configure_with(|_, ops, _| { ops.depend_on("clean"); Ok(()) });
    }

    #[test]
    fn project_name_based_on_directory() {
        let path = PathBuf::from("parent_dir/ProjectName");
        let project = Project::in_dir(path).unwrap();

        assert_eq!(project.id(), "ProjectName");
    }

    #[test]
    fn create_files_in_project() {
        let temp_dir =TempDir::new_in(env::current_dir().unwrap()).unwrap();
        assert!(temp_dir.path().exists());
        let project = Project::in_dir_with_id(temp_dir, "root").unwrap();
        let file = project.file("test1").expect("Couldn't make file from &str");
        assert_eq!(file.path(), project.project_dir().join("test1"));
        let file = project.file("test2".to_string()).expect("Couldn't make file from String");
        assert_eq!(file.path(), project.project_dir().join("test2"));
    }
}
