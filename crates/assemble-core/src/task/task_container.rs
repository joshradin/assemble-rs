//! The task container

use super::Executable;

use crate::defaults::task::DefaultTask;
use crate::project::{Project, ProjectError};
use crate::task::{Configure, ExecutableTaskMut, GenericTaskOrdering, Task, TaskOptions, TaskOrdering};
use crate::utilities::try_;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, RwLock, Weak};
use itertools::Itertools;
use crate::identifier::{InvalidId, TaskId};

#[derive(Default)]
pub struct TaskContainer<T: Executable> {
    inner: Arc<RwLock<TaskContainerInner<T>>>,
}

impl<T: Executable> Clone for TaskContainer<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Executable> TaskContainer<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(TaskContainerInner {
                unresolved_tasks: HashMap::new(),
                resolved_tasks: HashMap::new(),
            })),
        }
    }
}

impl<T: Executable + Send + Sync> TaskContainer<T> {
    pub fn register_task<N: 'static + Task<ExecutableTask = T>>(
        &mut self,
        task_id: TaskId,
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

    pub fn get_tasks(&self) -> Vec<TaskId> {
        let inner = self.inner.read().unwrap();
        let mut output = vec![];
        output.extend(inner.unresolved_tasks.keys().cloned());
        output.extend(inner.resolved_tasks.keys().cloned());
        (output)
    }

    /// Configures a task and gets some information about the task
    pub fn configure_task(
        &self,
        task: TaskId,
        project: &Project,
    ) -> Result<ConfiguredInfo, ProjectError> {
        {
            let read_guard = self.inner.read().unwrap();
            if read_guard.resolved_tasks.contains_key(&task) {
                let (_, info) = read_guard.resolved_tasks.get(&task).unwrap();
                return Ok(info.clone());
            }
        }

        let resolvable = {
            let mut write_guard = self.inner.write().unwrap();

            if let Some(resolvable) = write_guard.unresolved_tasks.remove(&task) {
                resolvable
            } else {
                return Err(ProjectError::IdentifierMissing(task));
            }
        };

        let resolved = resolvable.resolve_task(project)?;
        println!("resolved {:?}", resolved);
        let dependencies =resolved
            .task_dependencies()
            .into_iter()
            .map(|ordering| ordering.as_task_ids(project))
            .try_fold::<_, _, Result<_, ProjectError>>(
                Vec::new(),
                |mut accum, next| {
                    accum.extend(next?);
                    Ok(accum)
                }
            )?;

        let info = ConfiguredInfo::new(dependencies);
        {
            let mut write_guard = self.inner.write().unwrap();
            write_guard
                .resolved_tasks
                .insert(task, (resolved, info.clone()));
        }

        Ok(info)
    }

    /// Configures a task if hasn't been configured, then returns the fully configured Executable Task
    pub fn resolve_task(&mut self, task: TaskId, project: &Project) -> Result<T, ProjectError> {
        self.configure_task(task.clone(), project)?;
        let mut write_guard = self.inner.write().unwrap();
        Ok(write_guard.resolved_tasks.remove(&task).unwrap().0)
    }
}

#[derive(Default)]
struct TaskContainerInner<T: Executable> {
    unresolved_tasks: HashMap<TaskId, Box<(dyn ResolveTask<T> + Send + Sync)>>,
    resolved_tasks: HashMap<TaskId, (T, ConfiguredInfo)>,
}

pub struct TaskProvider<T: Task> {
    id: TaskId,
    inner: Arc<RwLock<TaskProviderInner<T>>>,
}

impl<T: Task> TaskProvider<T> {
    pub fn configure_with<F>(&mut self, config: F)
    where
        F: Fn(
                &mut T,
                &mut TaskOptions<T::ExecutableTask>,
                &Project,
            ) -> Result<(), ProjectError>
            + Send
            + Sync
            + 'static,
    {
        let mut lock = self.inner.write().unwrap();
        lock.configurations.push(Box::new(config));
    }
}

impl<T: Task, F> TaskConfigurator<T> for F
where
    for<'a> F: Fn(
        &'a mut T,
        &'a mut TaskOptions<T::ExecutableTask>,
        &'a Project,
    ) -> Result<(), ProjectError>,
    F: Send + Sync + 'static,
{
    fn configure_task(
        &self,
        task: &mut T,
        opts: &mut TaskOptions<T::ExecutableTask>,
        project: &Project,
    ) -> Result<(), ProjectError> {
        (self)(task, opts, project)
    }
}

pub trait TaskConfigurator<T: Task>: 'static + Send + Sync {
    fn configure_task(
        &self,
        task: &mut T,
        opts: &mut TaskOptions<T::ExecutableTask>,
        project: &Project,
    ) -> Result<(), ProjectError>;
}

pub type TaskConfiguratorObj<T> = dyn TaskConfigurator<T>;

struct TaskProviderInner<T: Task> {
    id: TaskId,
    c_pointer: Weak<RwLock<TaskContainerInner<T::ExecutableTask>>>,
    configurations: Vec<Box<dyn TaskConfigurator<T>>>,
}

trait ResolveTask<T: Executable> {
    fn resolve_task(&self, project: &Project) -> Result<T, ProjectError>;
}

impl<T: Task + 'static> ResolveTask<T::ExecutableTask> for Arc<RwLock<TaskProviderInner<T>>> {
    fn resolve_task(
        &self,
        project: &Project,
    ) -> Result<T::ExecutableTask, ProjectError> {
        let inner = self.read().unwrap();
        let (task, options) = try_::<_, ProjectError, _>(|| {
            let mut task = T::create();
            let mut options = TaskOptions::default();
            for configurator in &inner.configurations {
                configurator.configure_task(&mut task, &mut options, project)?;
            }
            Ok((task, options))
        })?;
        let mut output = task.into_task()?;
        output.set_task_id(inner.id.clone());
        options.apply_to(project, &mut output)?;
        Ok(output)
    }
}

assert_obj_safe!(ResolveTask<DefaultTask>);

/// Configured information about a task
#[derive(Debug, Clone)]
pub struct ConfiguredInfo {
    pub ordering: Vec<TaskOrdering<TaskId>>,
    _data: PhantomData<()>,
}


impl ConfiguredInfo {
    fn new(ordering: Vec<TaskOrdering<TaskId>>) -> Self {
        Self {
            ordering,
            _data: PhantomData,
        }
    }
}
