use crate::defaults::task::DefaultTask;
use crate::dependencies::Source;
use crate::task::task_container::{TaskContainer, TaskProvider};
use crate::task::{IntoTask, InvalidTaskIdentifier, Task, TaskIdentifier};
use std::path::Path;

pub struct Project<T: Task = DefaultTask> {
    task_container: TaskContainer<T>,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            task_container: Default::default(),
        }
    }
}

impl<T: Task> Project<T> {
    pub fn new() -> Self {
        Self {
            task_container: TaskContainer::new(),
        }
    }

    pub fn task<Task: 'static + IntoTask<Task = T>>(
        &mut self,
        id: &str,
    ) -> Result<TaskProvider<Task>> {
        let id = id.try_into()?;
        Ok(self.task_container.register_task(id))
    }

    pub fn registered_tasks(&self) {
        self.task_container.get_tasks();
    }

    /// Resolves a task by id
    pub fn resolve_task(&self, ids: &str) -> Result<Box<dyn Task>> {
        todo!()
    }

    pub fn sources(&self) -> impl IntoIterator<Item = &dyn Source> {
        vec![]
    }

    pub fn add_source<S: 'static + Source>(&mut self, source: S) {
        unimplemented!()
    }

    pub fn visitor<R, V: VisitProject<T, R>>(&self, visitor: &mut V) -> R {
        visitor.visit(self)
    }

    pub fn visitor_mut<R, V: VisitMutProject<T, R>>(&mut self, visitor: &mut V) -> R {
        visitor.visit_mut(self)
    }

    /// The directory of the project
    pub fn project_dir(&self) -> &Path {
        unimplemented!()
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
pub trait VisitProject<T: Task, R = ()> {
    /// Visit the project
    fn visit(&mut self, project: &Project<T>) -> R;
}

/// trait for visiting project thats mutable
pub trait VisitMutProject<T: Task, R = ()> {
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
