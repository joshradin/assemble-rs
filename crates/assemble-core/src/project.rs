use std::marker::PhantomData;
use crate::defaults::task::DefaultTask;
use crate::dependencies::Source;
use crate::task::task_container::{TaskContainer, TaskProvider};
use crate::task::{Empty, Task, InvalidTaskIdentifier, TaskIdentifier, ExecutableTask};
use crate::workspace::WorkspaceDirectory;
use crate::Workspace;
use std::path::{Path, PathBuf};

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
/// task_provider.configure(|_empty, opts, project| {
///     opts.do_first(|_, _| {
///         println!("Hello, World");
///         Ok(())
///     })
/// });
/// ```
pub struct Project<T: ExecutableTask = DefaultTask> {
    task_container: TaskContainer<T>,
    workspace: Workspace,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            task_container: Default::default(),
            ..Project::new()
        }
    }
}

impl<Executable: ExecutableTask> Project<Executable> {
    /// Create a new Project, with the current directory as the the directory to load
    pub fn new() -> Self {
        Self::in_dir(std::env::current_dir().unwrap())
    }

    /// Creates an assemble project in a specified directory.
    pub fn in_dir(path: impl AsRef<Path>) -> Self {
        Self {
            task_container: TaskContainer::new(),
            workspace: Workspace::new(path),
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

    pub fn registered_tasks(&self) {
        self.task_container.get_tasks();
    }

    /// Resolves a task by id
    pub fn resolve_task(&self, ids: &str) -> Result<Box<dyn ExecutableTask>> {
        todo!()
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
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error(transparent)]
    InvalidIdentifier(#[from] InvalidTaskIdentifier),
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
    use crate::defaults::task::Echo;
    use crate::project::Project;
    use crate::task::task_container::TaskContainer;

    #[test]
    fn create_tasks() {
        let mut project = Project::default();

        let mut provider = project.task::<Echo>("tasks").unwrap();
        provider.configure(|echo, _, _| echo.string = "Hello, World!".to_string());
        provider.configure(|_, ops, _| ops.depend_on("clean"))
    }
}
