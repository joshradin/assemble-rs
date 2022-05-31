//! The task container

use super::{Task, TaskIdentifier};

use crate::defaults::tasks::DefaultTask;
use crate::project::Project;
use crate::task::{IntoTask, TaskMut, TaskOptions};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};

#[derive(Default)]
pub struct TaskContainer<T: Task> {
    inner: Arc<RwLock<TaskContainerInner<T>>>,
}

impl<T: Task> TaskContainer<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(TaskContainerInner {
                unresolved_tasks: HashMap::new(),
                resolved_tasks: HashMap::new(),
            })),
        }
    }
}

impl<T: Task> TaskContainer<T> {
    pub fn register_task<N: 'static + IntoTask<Task = T>>(
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
struct TaskContainerInner<T: Task> {
    unresolved_tasks: HashMap<TaskIdentifier, Box<(dyn ResolveTask<T>)>>,
    resolved_tasks: HashMap<TaskIdentifier, T>,
}

pub struct TaskProvider<T: IntoTask> {
    id: TaskIdentifier,
    inner: Arc<RwLock<TaskProviderInner<T>>>,
}

impl<T: IntoTask> TaskProvider<T> {
    pub fn configure<F: 'static + Fn(&mut T, &mut TaskOptions, &Project)>(&mut self, config: F) {
        let mut lock = self.inner.write().unwrap();
        lock.configurations.push(Box::new(config));
    }
}

pub type TaskConfigurator<T> = dyn Fn(&mut T, &mut TaskOptions, &Project);

struct TaskProviderInner<T: IntoTask> {
    id: TaskIdentifier,
    c_pointer: Weak<RwLock<TaskContainerInner<T::Task>>>,
    configurations: Vec<Box<TaskConfigurator<T>>>,
}

trait ResolveTask<T: Task> {
    fn resolve_task(self, project: &Project) -> T;
}

impl<T: IntoTask> ResolveTask<T::Task> for Arc<RwLock<TaskProviderInner<T>>> {
    fn resolve_task(self, project: &Project) -> T::Task {
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