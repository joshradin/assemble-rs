use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use crate::{BuildResult, Executable, Project};
use crate::identifier::TaskId;
use crate::project::buildable::{Buildable, BuiltByContainer, IntoBuildable};
use crate::project::{ProjectError, ProjectResult, SharedProject};
use crate::task::Empty;
use super::ExecutableTask;
use super::Task;

pub struct ConfigureTask<T : Task> {
    func: Box<dyn FnOnce(&mut Executable<T>, &Project) -> ProjectResult + Send>
}

impl<T : Task> ConfigureTask<T> {

    pub fn configure(self, task: &mut Executable<T>, project: &Project) -> ProjectResult {
        (self.func)(task, project)
    }
}

pub trait ResolveTask {
    type Executable: ExecutableTask;

    fn resolve_task(self, project: &Project) -> ProjectResult<Self::Executable>;
}


pub struct LazyTask<T : Task + Send + Debug + 'static> {
    task_type: PhantomData<T>,
    task_id: TaskId,
    configurations: Vec<ConfigureTask<T>>,
    shared: Option<SharedProject>,
}

impl< T: Task + Send + Debug + 'static> LazyTask<T> {

    fn empty() -> Self {
        Self {
            task_type: PhantomData,
            task_id: TaskId::default(),
            configurations: vec![],
            shared: None
        }
    }
}

impl<T: Task + Send + Debug + 'static> ResolveTask for LazyTask<T> {
    type Executable = Executable<T>;

    fn resolve_task(self, project: &Project) -> ProjectResult<Executable<T>> {
        let task = T::new(project);
        let mut executable = Executable::new(self.shared.unwrap().clone(), task, self.task_id);
        for config in self.configurations {
            config.configure(&mut executable, project)?;
        }

        Ok(executable)
    }
}


enum TaskProviderInner<T : Task + Send + Debug + 'static> {
    Lazy(LazyTask<T>),
    Configured(Executable<T>),
}

impl<T: Task + Send + Debug + 'static> TaskProviderInner<T> {

    fn resolve(&mut self, project: &Project) -> ProjectResult<()> {
        let configured: Executable<T> = match self {
            TaskProviderInner::Lazy(lazy) => {
                let lazy = std::mem::replace(lazy, LazyTask::empty());
                lazy.resolve_task(project)?
            }
            TaskProviderInner::Configured { .. } => { return Ok(())}
        };
        *self = TaskProviderInner::Configured(configured);
        Ok(())
    }

    fn configured(&mut self, project: &Project) -> ProjectResult<&Executable<T>> {
        self.resolve(project)?;
        if let TaskProviderInner::Configured(exec) = &*self {
            Ok(exec)
        } else {
            panic!("task should be configured")
        }

    }
}



pub struct TaskProvider<T: Task + Send + Debug + 'static> {
    id: TaskId,
    connection: Arc<Mutex<TaskProviderInner<T>>>
}

impl<T: Task + Send + Debug + 'static> TaskProvider<T> {

    pub fn configure_with<F>(&mut self, config: F) -> ProjectResult
        where F : FnOnce(&mut Executable<T>, &Project) -> ProjectResult + Send + 'static
    {
        let mut guard = self.connection.lock()?;
        match &mut *guard {
            TaskProviderInner::Lazy(lazy) => {
                lazy.configurations.push(ConfigureTask { func: Box::new(config) });
            }
            TaskProviderInner::Configured(c) => {
                let shared_project = c.project().clone();
                let project = shared_project.lock()?;
                config(c, &*project)?;
            }
        }
        Ok(())
    }
}

assert_impl_all!(TaskProvider<Empty>: Sync);

impl<T: Task + Send + Debug + 'static> Debug for TaskProvider<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskProvider")
            .field("id", &self.id)
            .finish()
    }
}


impl<T: Task + Send + Debug + 'static> Buildable for TaskProvider<T> {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        let mut guard = self.connection.lock()?;
        let configured = guard.configured(project)?;
        configured.into_buildable().get_dependencies(project)
    }
}

impl<T: Task + Send + Debug + 'static> ExecutableTask for TaskProvider<T> {
    fn task_id(&self) -> &TaskId {
        &self.id
    }

    fn execute(&mut self, project: &Project) -> BuildResult {
        todo!()
    }
}

impl<T: Task + Send + Debug + 'static> Clone for TaskProvider<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            connection: self.connection.clone()
        }
    }
}

