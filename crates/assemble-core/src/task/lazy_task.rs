use super::ExecutableTask;
use super::Task;
use crate::exception::BuildException;
use crate::identifier::{InvalidId, TaskId};
use crate::project::buildable::{Buildable, BuiltByContainer, IntoBuildable};
use crate::project::{ProjectError, ProjectResult, SharedProject};
use crate::properties::Provides;
use crate::task::{BuildableTask, Empty, HasTaskId};
use crate::{BuildResult, Executable, Project};
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

pub struct ConfigureTask<T: Task> {
    func: Box<dyn FnOnce(&mut Executable<T>, &Project) -> ProjectResult + Send>,
}

impl<T: Task> ConfigureTask<T> {
    pub fn configure(self, task: &mut Executable<T>, project: &Project) -> ProjectResult {
        (self.func)(task, project)
    }
}

pub trait ResolveTask {
    type Executable: ExecutableTask;

    fn resolve_task(self, project: &SharedProject) -> ProjectResult<Self::Executable>;
}

pub trait ResolveInnerTask: Send {
    fn resolve_task(&mut self, project: &SharedProject) -> ProjectResult<()>;
}

assert_obj_safe!(ResolveInnerTask);

pub struct LazyTask<T: Task + Send + Debug + 'static> {
    task_type: PhantomData<T>,
    task_id: TaskId,
    configurations: Vec<ConfigureTask<T>>,
    shared: Option<SharedProject>,
}

impl<T: Task + Send + Debug + 'static> LazyTask<T> {

    fn new(id: TaskId, shared: &SharedProject) -> Self {
        Self {
            task_type: PhantomData,
            task_id: id,
            configurations: vec![],
            shared: Some(shared.clone())
        }
    }

    fn empty() -> Self {
        Self {
            task_type: PhantomData,
            task_id: TaskId::default(),
            configurations: vec![],
            shared: None,
        }
    }
}

impl<T: Task + Send + Debug + 'static> ResolveTask for LazyTask<T> {
    type Executable = Executable<T>;

    fn resolve_task(self, project: &SharedProject) -> ProjectResult<Executable<T>> {
        let task = project.with(T::new)?;
        let mut executable = Executable::new(self.shared.unwrap().clone(), task, self.task_id);
        for config in self.configurations {
            project.with(|project| config.configure(&mut executable, project))?;
        }

        Ok(executable)
    }
}

enum TaskHandleInner<T: Task + Send + Debug + 'static> {
    Lazy(LazyTask<T>),
    Configured(Executable<T>),
}

impl<T: Task + Send + Debug + 'static> TaskHandleInner<T> {
    fn bare_resolve(&mut self) -> ProjectResult<()> {
        let project = match self {
            TaskHandleInner::Lazy(l) => l
                .shared
                .as_ref()
                .ok_or(ProjectError::NoSharedProjectSet)?
                .clone(),
            TaskHandleInner::Configured(_) => {
                return Ok(());
            }
        };
        self.resolve(&project)
    }

    fn resolve(&mut self, project: &SharedProject) -> ProjectResult<()> {
        let configured: Executable<T> = match self {
            TaskHandleInner::Lazy(lazy) => {
                let lazy = std::mem::replace(lazy, LazyTask::empty());
                lazy.resolve_task(project)?
            }
            TaskHandleInner::Configured { .. } => return Ok(()),
        };
        *self = TaskHandleInner::Configured(configured);
        Ok(())
    }

    fn bare_configured(&mut self) -> ProjectResult<&Executable<T>> {
        self.bare_resolve()?;
        if let TaskHandleInner::Configured(exec) = &*self {
            Ok(exec)
        } else {
            panic!("task should be configured")
        }
    }

    fn configured(&mut self, project: &SharedProject) -> ProjectResult<&Executable<T>> {
        self.resolve(project)?;
        if let TaskHandleInner::Configured(exec) = &*self {
            Ok(exec)
        } else {
            panic!("task should be configured")
        }
    }

    fn configured_mut(&mut self, project: &SharedProject) -> ProjectResult<&mut Executable<T>> {
        self.resolve(project)?;
        if let TaskHandleInner::Configured(exec) = &mut *self {
            Ok(exec)
        } else {
            panic!("task should be configured")
        }
    }
}

impl<T: Task + Send + Debug + 'static> ResolveInnerTask for TaskHandleInner<T> {
    fn resolve_task(&mut self, project: &SharedProject) -> ProjectResult<()> {
        self.resolve(project)
    }
}

pub struct TaskHandle<T: Task + Send + Debug + 'static> {
    id: TaskId,
    connection: Arc<Mutex<TaskHandleInner<T>>>,
}

impl<T: Task + Send + Debug + 'static> TaskHandle<T> {
    pub fn configure_with<F>(&mut self, config: F) -> ProjectResult
    where
        F: FnOnce(&mut Executable<T>, &Project) -> ProjectResult + Send + 'static,
    {
        let mut guard = self.connection.lock()?;
        match &mut *guard {
            TaskHandleInner::Lazy(lazy) => {
                lazy.configurations.push(ConfigureTask {
                    func: Box::new(config),
                });
            }
            TaskHandleInner::Configured(c) => {
                let shared_project = c.project().clone();
                shared_project.with(|project| (config)(c, project))?;
            }
        }
        Ok(())
    }

    pub fn provides<F, R>(&self, func: F) -> TaskProvider<T, R, F>
        where
        F: Fn(&Executable<T>) -> R + Send + Sync,
        R : Clone + Send + Sync {
        TaskProvider::new(self.clone(), func)
    }
}

assert_impl_all!(TaskHandle<Empty>: Sync);

impl<T: Task + Send + Debug + 'static> Debug for TaskHandle<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskProvider")
            .field("id", &self.id)
            .finish()
    }
}

impl<T: Task + Send + Debug + 'static> Buildable for TaskHandle<T> {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        let mut guard = self.connection.lock()?;
        let configured = guard.configured(&project.as_shared())?;
        configured.into_buildable().get_dependencies(project)
    }
}

impl<T: Task + Send + Debug + 'static> HasTaskId for TaskHandle<T> {
    fn task_id(&self) -> &TaskId {
        &self.id
    }
}

impl<T: Task + Send + Debug + 'static> BuildableTask for TaskHandle<T> {
    fn built_by(&self, project: &Project) -> BuiltByContainer {
        todo!()
    }
}

impl<T: Task + Send + Debug + 'static> Clone for TaskHandle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            connection: self.connection.clone(),
        }
    }
}

impl<T: Task + Send + Debug + 'static> ResolveInnerTask for TaskHandle<T> {
    fn resolve_task(&mut self, project: &SharedProject) -> ProjectResult<()> {
        self.connection.lock()?.resolve_task(project)
    }
}
impl<T: Task + Send + Debug + 'static> ExecutableTask for TaskHandle<T> {
    fn execute(&mut self, project: &Project) -> BuildResult {
        let mut guard = self
            .connection
            .lock()
            .map_err(|_| BuildException::new("Could not get access to provider"))?;
        guard.configured_mut(&project.as_shared())?.execute(project)
    }
}

pub trait ResolveExecutable: ResolveInnerTask {
    fn get_executable(&mut self, project: &SharedProject)
        -> ProjectResult<Box<dyn ExecutableTask>>;
}

impl<T: Task + Send + Debug + 'static> ResolveExecutable for TaskHandle<T> {
    fn get_executable(
        &mut self,
        project: &SharedProject,
    ) -> ProjectResult<Box<dyn ExecutableTask>> {
        self.resolve_task(project)?;
        Ok(Box::new(self.clone()))
    }
}

pub struct TaskProvider<T, R, F>
where
    T: Task + Send + Debug + 'static,
    F: Fn(&Executable<T>) -> R + Send + Sync,
    R : Clone + Send + Sync
{
    handle: TaskHandle<T>,
    lift: F,
}

impl<T, F, R> Provides<R> for TaskProvider<T, R, F> where
    T: Task + Send + Debug + 'static,
    F: Fn(&Executable<T>) -> R + Send + Sync,
    R : Clone + Send + Sync{
    fn try_get(&self) -> Option<R> {
        let mut guard = self.handle.connection.lock().expect("Could not get inner");
        let configured = guard.bare_configured().expect("could not configure task");
        Some((self.lift)(configured))
    }
}

impl<T, F, R> TaskProvider<T, R, F>
where
    T: Task + Send + Debug + 'static,
    F: Fn(&Executable<T>) -> R + Send + Sync,
    R : Clone + Send + Sync
{
    pub fn new(handle: TaskHandle<T>, lift: F) -> Self {
        Self { handle, lift }
    }
}


#[derive(Debug)]
pub struct TaskHandleFactory {
    project: SharedProject
}

impl TaskHandleFactory {

    pub fn new(project: SharedProject) -> Self {
        Self { project }
    }

    /// Creates a task handle that's not configured
    pub fn create_handle<T : Task + Send + Debug + 'static>(&self, id: &str) -> Result<TaskHandle<T>, InvalidId> {
        let task_id = TaskId::new(id)?;
        let lazy = LazyTask::<T>::new(task_id.clone(), &self.project);
        let inner = TaskHandleInner::Lazy(lazy);
        Ok(TaskHandle {
            id: task_id,
            connection: Arc::new(Mutex::new(inner))
        })
    }
}


#[cfg(test)]
mod tests {

    #[test]
    fn lazy_create_task() {}
}
