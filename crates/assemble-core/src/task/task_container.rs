//! The task container

use super::{ExecutableTask, TaskIdentifier};

use crate::defaults::task::DefaultTask;
use crate::project::Project;
use crate::task::{ExecutableTaskMut, Task, TaskOptions};
use once_cell::sync::Lazy;
use std::collections::HashMap;
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

impl<T: ExecutableTask> TaskContainer<T> {
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
        let boxed: Box<dyn ResolveTask<T>> = Box::new(task_inner_clone);

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

    pub fn get_task(&self, task: TaskIdentifier, project: &Project) -> Option<&T> {
        todo!()
    }
}

#[derive(Default)]
struct TaskContainerInner<T: ExecutableTask> {
    unresolved_tasks: HashMap<TaskIdentifier, Box<(dyn ResolveTask<T>)>>,
    resolved_tasks: HashMap<TaskIdentifier, T>,
}

pub struct TaskProvider<T: Task> {
    id: TaskIdentifier,
    inner: Arc<RwLock<TaskProviderInner<T>>>,
}

impl<T: Task> TaskProvider<T> {
    pub fn configure<F: 'static + Fn(&mut T, &mut TaskOptions, &Project)>(&mut self, config: F) {
        let mut lock = self.inner.write().unwrap();
        lock.configurations.push(Box::new(config));
    }
}

pub type TaskConfigurator<T, E> = dyn Fn(&mut T, &mut TaskOptions, &Project);

struct TaskProviderInner<T: Task> {
    id: TaskIdentifier,
    c_pointer: Weak<RwLock<TaskContainerInner<T::ExecutableTask>>>,
    configurations: Vec<Box<TaskConfigurator<T>>>,
}

trait ResolveTask<T: ExecutableTask> {
    fn resolve_task(self, project: &Project) -> T;
}

impl<T: Task> ResolveTask<T::ExecutableTask> for Arc<RwLock<TaskProviderInner<T>>> {
    fn resolve_task(self, project: &Project) -> T::ExecutableTask {
        let inner = self.read().unwrap();
        let mut task = T::create();
        let mut options = TaskOptions::default();
        for configurator in &inner.configurations {
            (configurator)(&mut task, &mut options, project)
        }
        let mut output = task.into_task().map_err(|_| ()).unwrap();
        output.set_task_id(inner.id.clone());
        options.apply_to(project, &mut output);
        output
    }
}

assert_obj_safe!(ResolveTask<DefaultTask>);
