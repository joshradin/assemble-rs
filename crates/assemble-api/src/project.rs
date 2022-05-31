use crate::defaults::tasks::DefaultTask;
use crate::dependencies::Source;
use crate::task::task_container::{TaskContainer, TaskProvider};
use crate::task::{IntoTask, InvalidTaskIdentifier, Task, TaskIdentifier};

#[derive(Default)]
pub struct Project<T: Task = DefaultTask> {
    task_container: TaskContainer<T>,
}

impl<T: Task> Project<T> {
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

    pub fn resolve_task(&self, ids: &str) {}

    pub fn sources(&self) -> Vec<&dyn Source> {
        unimplemented!()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error(transparent)]
    InvalidIdentifier(#[from] InvalidTaskIdentifier),
}

type Result<T> = std::result::Result<T, ProjectError>;

#[cfg(test)]
mod test {
    use crate::defaults::tasks::Echo;
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
