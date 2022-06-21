//! The task container

use super::{ExecutableTask, TaskIdentifier};

use crate::defaults::task::DefaultTask;
use crate::project::{Project, ProjectError};
use crate::task::{ExecutableTaskMut, InvalidTaskIdentifier, Task, TaskOptions, TaskOrdering};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, RwLock, Weak};

#[derive(Default)]
pub struct TaskContainer<T: ExecutableTask> {
    inner: Arc<RwLock<TaskContainerInner<T>>>,
}

impl<T: ExecutableTask> TaskContainer<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(TaskContainerInner {
                unresolved_tasks: HashMap::new(),
                resolved_tasks: HashMap::new(),
            })),
        }
    }
}

impl<T: ExecutableTask + Send + Sync> TaskContainer<T> {
    pub fn register_task<N: 'static + Task<ExecutableTask = T>>(
        &mut self,
        task_id: TaskIdentifier,
    ) -> TaskProvider<N> {
        let inner_container = self.inner.clone();

        let inner_task_provider = Arc::new(RwLock::new(TaskProviderInner::<N> {
            id: task_id.clone(),
            c_pointer: Arc::downgrade(&inner_container),
            configurations: vec![],
        }));

        let task_inner_clone = inner_task_provider.clone();
        let boxed: Box<dyn ResolveTask<T> + Send + Sync> = Box::new(task_inner_clone);

        let mut inner_guard = self.inner.write().unwrap();
        let map = &mut inner_guard.unresolved_tasks;
        map.insert(task_id.clone(), boxed);

        TaskProvider {
            id: task_id,
            inner: inner_task_provider,
        }
    }

    pub fn get_tasks(&self) -> Vec<TaskIdentifier> {
        todo!()
    }

    /// Configures a task and gets some information about the task
    pub fn configure_task(&mut self, task: TaskIdentifier, project: &Project<T>) -> Result<ConfiguredInfo, ProjectError> {
        let mut write_guard = self.inner.write().unwrap();
        if write_guard.resolved_tasks.contains_key(&task) {
            let (_, info) = write_guard.resolved_tasks.get(&task).unwrap();
            return Ok(info.clone());
        }

        if let Some(resolvable) = write_guard.unresolved_tasks.remove(&task) {
            let resolved = resolvable.resolve_task(project)?;
            let dependencies = Vec::from_iter(resolved.task_dependencies().into_iter().cloned());

            let info = ConfiguredInfo::new(dependencies);
            write_guard.resolved_tasks.insert(task, (resolved, info.clone()));
            Ok(info)
        } else {
            Err(ProjectError::IdentifierMissing(task))
        }

    }

    /// Configures a task if hasn't been configured, then returns the fully configured Executable Task
    pub fn resolve_task(&mut self, task: TaskIdentifier, project: &Project<T>) -> Result<T, ProjectError> {
        self.configure_task(task.clone(), project)?;
        let mut write_guard = self.inner.write().unwrap();
        Ok(write_guard.resolved_tasks.remove(&task).unwrap().0)
    }
}

#[derive(Default)]
struct TaskContainerInner<T: ExecutableTask> {
    unresolved_tasks: HashMap<TaskIdentifier, Box<(dyn ResolveTask<T> + Send + Sync)>>,
    resolved_tasks: HashMap<TaskIdentifier, (T, ConfiguredInfo)>,
}

pub struct TaskProvider<T: Task> {
    id: TaskIdentifier,
    inner: Arc<RwLock<TaskProviderInner<T>>>,
}

impl<T: Task> TaskProvider<T> {
    pub fn configure<F: TaskConfigurator<T>>(&mut self, config: F)
    {
        let mut lock = self.inner.write().unwrap();
        lock.configurations.push(Box::new(config));
    }
}

impl<T : Task, F> TaskConfigurator<T> for F
    where F : Fn(&mut T, &mut TaskOptions<T::ExecutableTask>, &Project<T::ExecutableTask>) -> Result<(), ProjectError>,
        F : Send + Sync + 'static {
    fn configure_task(&self, task: &mut T, opts: &mut TaskOptions<T::ExecutableTask>, project: &Project<T::ExecutableTask>) -> Result<(), ProjectError> {
        (self)(task, opts, project)
    }
}


pub trait TaskConfigurator<T : Task> : 'static + Send + Sync {
    fn configure_task(&self, task: &mut T, opts: &mut TaskOptions<T::ExecutableTask>, project: &Project<T::ExecutableTask>) -> Result<(), ProjectError>;
}

pub type TaskConfiguratorObj<T> = dyn TaskConfigurator<T>;

struct TaskProviderInner<T: Task> {
    id: TaskIdentifier,
    c_pointer: Weak<RwLock<TaskContainerInner<T::ExecutableTask>>>,
    configurations: Vec<Box<dyn TaskConfigurator<T>>>,
}

trait ResolveTask<T: ExecutableTask> {
    fn resolve_task(&self, project: &Project<T>) -> Result<T, ProjectError>;
}

impl<T: Task + 'static> ResolveTask<T::ExecutableTask> for Arc<RwLock<TaskProviderInner<T>>> {
    fn resolve_task(&self, project: &Project<T::ExecutableTask>) -> Result<T::ExecutableTask, ProjectError> {
        let inner = self.read().unwrap();
        let mut task = T::create();
        let mut options = TaskOptions::default();
        for configurator in &inner.configurations {
            configurator.configure_task(&mut task, &mut options, project)?;
        }
        let mut output = task.into_task()?;
        output.set_task_id(inner.id.clone());
        options.apply_to(project, &mut output);
        Ok(output)
    }
}

assert_obj_safe!(ResolveTask<DefaultTask>);

/// Configured information about a task
#[derive(Clone, Debug)]
pub struct ConfiguredInfo {
    pub ordering: Vec<TaskOrdering>,
    _data: PhantomData<()>
}

impl ConfiguredInfo {
    fn new(ordering: Vec<TaskOrdering>) -> Self {
        Self { ordering, _data: PhantomData }
    }
}
