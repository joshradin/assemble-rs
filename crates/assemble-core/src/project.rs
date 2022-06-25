use std::any::Any;
use std::io;
use crate::defaults::task::DefaultTask;
use crate::dependencies::Source;
use crate::task::task_container::{TaskContainer, TaskProvider};
use crate::task::{Empty, ExecutableTask, InvalidTaskIdentifier, Task, TaskId};
use crate::workspace::WorkspaceDirectory;
use crate::{BuildResult, Workspace};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use crate::exception::BuildException;
use crate::plugins::{Plugin, PluginError, ToPlugin};

pub mod configuration;

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
/// let mut project = Project::new();
/// let mut task_provider = project.task::<Empty>("hello_world").expect("Couldn't create 'hello_task'");
/// task_provider.configure_with(|_empty, opts, project| {
///     opts.do_first(|_, _| {
///         println!("Hello, World");
///         Ok(())
///     });
///     Ok(())
/// });
/// ```
pub struct Project<T: ExecutableTask> {
    task_container: TaskContainer<T>,
    workspace: Workspace,
    applied_plugins: Vec<String>
}

impl Default for Project<DefaultTask> {
    fn default() -> Self {
        Self {
            task_container: Default::default(),
            ..Project::new()
        }
    }
}

impl<Executable: ExecutableTask + Send + Sync> Project<Executable> {
    /// Create a new Project, with the current directory as the the directory to load
    pub fn new() -> Self {
        Self::in_dir(std::env::current_dir().unwrap())
    }

    /// Creates an assemble project in a specified directory.
    pub fn in_dir(path: impl AsRef<Path>) -> Self {
        Self {
            task_container: TaskContainer::new(),
            workspace: Workspace::new(path),
            applied_plugins: Default::default()
        }
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
    pub fn task<T: 'static + Task<ExecutableTask = Executable>>(
        &mut self,
        id: &str,
    ) -> Result<TaskProvider<T>> {
        let id = id.try_into()?;
        Ok(self.task_container.register_task(id))
    }

    pub fn registered_tasks(&self) -> Vec<TaskId> {
        self.task_container.get_tasks()
    }

    /// Try to resolve a task id
    pub fn resolve_task_id(&self, id: &str) -> Result<TaskId> {
        let potential = self.task_container.get_tasks()
            .into_iter()
            .filter(|task_id| task_id.is_valid_representation(id))
            .collect::<Vec<_>>();

        match &potential[..] {
            [] => Err(ProjectError::InvalidIdentifier(InvalidTaskIdentifier(id.to_string()))),
            [once] => Ok(once.clone()),
            alts => panic!("Many found for {}: {:?}", id, alts)
        }
    }



    pub fn sources(&self) -> impl IntoIterator<Item = &dyn Source> {
        vec![]
    }

    pub fn add_source<S: 'static + Source>(&mut self, source: S) {
        unimplemented!()
    }

    pub fn visitor<R, V: VisitProject<Executable, R>>(&self, visitor: &mut V) -> R {
        visitor.visit(self)
    }

    pub fn visitor_mut<R, V: VisitMutProject<Executable, R>>(&mut self, visitor: &mut V) -> R {
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

    pub fn apply_plugin<P : Plugin<Executable>>(&mut self, plugin: P) -> Result<()> {
        plugin.apply(self).map_err(ProjectError::from)
    }

    pub fn plugin<P : ToPlugin<Executable>>(&self, p: P) -> Result<P::Plugin> {
        p.to_plugin(self).map_err(ProjectError::from)
    }

    /// Get access to the task container
    pub fn task_container(&self) -> TaskContainer<Executable> {
        self.task_container.clone()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("Identifier Missing: {0}")]
    IdentifierMissing(TaskId),
    #[error(transparent)]
    InvalidIdentifier(#[from] InvalidTaskIdentifier),
    #[error(transparent)]
    PluginError(#[from] PluginError),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error("Inner Error {:?} {{ ... }}", inner.type_id())]
    SomeError { inner: Box<dyn Any + Send> }
}

impl From<Box<dyn Any + Send>> for ProjectError {
    fn from(e: Box<dyn Any + Send>) -> Self {
        Self::SomeError { inner: e }
    }
}

type Result<T> = std::result::Result<T, ProjectError>;

///  trait for visiting projects
pub trait VisitProject<T: ExecutableTask, R = ()> {
    /// Visit the project
    fn visit(&mut self, project: &Project<T>) -> R;
}

/// trait for visiting project thats mutable
pub trait VisitMutProject<T: ExecutableTask, R = ()> {
    /// Visit a mutable project.
    fn visit_mut(&mut self, project: &mut Project<T>) -> R;
}



#[cfg(test)]
mod test {
    use crate::project::Project;
    use crate::task::Empty;
    use crate::task::task_container::TaskContainer;

    #[test]
    fn create_tasks() {
        let mut project = Project::default();

        let mut provider = project.task::<Empty>("tasks").unwrap();
        provider.configure_with(|_, ops, _| { ops.depend_on("clean"); Ok(()) })
    }
}
