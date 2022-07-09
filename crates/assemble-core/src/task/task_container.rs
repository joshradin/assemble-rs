//! The task container

use super::Executable;

use crate::defaults::task::DefaultTask;
use crate::identifier::{InvalidId, TaskId};
use crate::project::buildable::{Buildable, IntoBuildable};
use crate::project::{Project, ProjectError};
use crate::task::{
    Configure, ExecutableTaskMut, GenericTaskOrdering, Task, TaskOptions, TaskOrdering,
};
use crate::utilities::try_;
use crate::BuildResult;
use itertools::Itertools;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::sync::{Arc, RwLock, RwLockReadGuard, Weak};
use crate::properties::Provides;

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
                taken_tasks: HashMap::new(),
                in_process: HashSet::new(),
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
        output.extend(inner.taken_tasks.keys().cloned());
        output.extend(inner.in_process.iter().cloned());
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
                println!("Got already resolved task: {}", task);
                let (_, info) = read_guard.resolved_tasks.get(&task).unwrap();
                return Ok(info.clone());
            } else if read_guard.taken_tasks.contains_key(&task) {
                println!("Got already taken task: {}", task);
                let info = read_guard.taken_tasks.get(&task).unwrap();
                return Ok(info.clone());
            } else if read_guard.in_process.contains(&task) {
                println!("Attempting to get in process task: {}", task);
                return Ok(ConfiguredInfo::new(vec![]));
            }
            println!("{} needs to be resolved", task);
        }

        let resolvable = {
            let mut write_guard = self.inner.write().unwrap();
            println!(
                "Unresolved tasks: {:#?}",
                write_guard.unresolved_tasks.keys()
            );
            let out = if let Some(resolvable) = write_guard.unresolved_tasks.remove(&task) {
                println!("Resolving task {}", task);
                resolvable
            } else {
                return Err(ProjectError::IdentifierMissing(task));
            };
            write_guard.in_process.insert(task.clone());
            out
        };

        let resolved = resolvable.resolve_task(project)?;
        println!("resolved {:?}", resolved);
        let dependencies = resolved
            .task_dependencies()
            .into_iter()
            .map(|ordering| {
                println!("Attempting to get id of ordering {:?}", ordering);
                ordering.as_task_ids(project)
            })
            .try_fold::<_, _, Result<_, ProjectError>>(Vec::new(), |mut accum, next| {
                match next {
                    Ok(found) => {
                        accum.extend(found);
                    }
                    Err(e) => return Err(e),
                }
                Ok(accum)
            })?;
        println!("{:?} dependencies: {:?}", resolved, dependencies);
        let info = ConfiguredInfo::new(dependencies);
        {
            let mut write_guard = self.inner.write().unwrap();
            write_guard.in_process.remove(&task);
            write_guard
                .resolved_tasks
                .insert(task, (resolved, info.clone()));
        }

        Ok(info)
    }

    /// Configures a task if hasn't been configured, then returns the fully configured Executable Task
    pub fn resolve_task(&mut self, task: TaskId, project: &Project) -> Result<T, ProjectError> {
        let config = self.configure_task(task.clone(), project)?;
        let mut write_guard = self.inner.write().unwrap();
        write_guard.taken_tasks.insert(task.clone(), config);
        Ok(write_guard.resolved_tasks.remove(&task).unwrap().0)
    }

    /// Configures a task if hasn't been configured, then returns the fully configured Executable Task
    pub fn resolved_ref_task(
        &mut self,
        task: TaskId,
        project: &Project,
    ) -> Result<TaskReference<T>, ProjectError> {
        self.configure_task(task.clone(), project)?;
        let read_guard = self.inner.read().unwrap();
        Ok(TaskReference::new(read_guard, task))
    }
}

pub struct TaskReference<'r, T: Executable> {
    guard: RwLockReadGuard<'r, TaskContainerInner<T>>,
    task_id: TaskId,
}

impl<'r, T: Executable> AsRef<T> for TaskReference<'r, T> {
    fn as_ref(&self) -> &T {
        self.guard
            .resolved_tasks
            .get(&self.task_id)
            .map(|s| &s.0)
            .unwrap()
    }
}

impl<'r, T: Executable> TaskReference<'r, T> {
    fn new(guard: RwLockReadGuard<'r, TaskContainerInner<T>>, id: TaskId) -> Self {
        Self { guard, task_id: id }
    }
}

#[derive(Default)]
struct TaskContainerInner<T: Executable> {
    unresolved_tasks: HashMap<TaskId, Box<(dyn ResolveTask<T> + Send + Sync)>>,
    resolved_tasks: HashMap<TaskId, (T, ConfiguredInfo)>,
    taken_tasks: HashMap<TaskId, ConfiguredInfo>,
    in_process: HashSet<TaskId>,
}

pub struct TaskProvider<T: Task> {
    id: TaskId,
    inner: Arc<RwLock<TaskProviderInner<T>>>,
}

impl<T: Task> TaskProvider<T> {
    pub fn id(&self) -> TaskId {
        self.id.clone()
    }
}

impl<T: Task> Clone for TaskProvider<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            inner: self.inner.clone(),
        }
    }
}

impl<T: Task> IntoBuildable for &TaskProvider<T> {
    type Buildable = TaskId;

    fn into_buildable(self) -> Self::Buildable {
        self.id.clone()
    }
}

impl<T: Task> IntoBuildable for TaskProvider<T> {
    type Buildable = TaskId;

    fn into_buildable(self) -> Self::Buildable {
        self.id
    }
}

impl<T: Task> Debug for TaskProvider<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.id)
    }
}

impl<T: Task> TaskProvider<T> {
    /// Add some configuration to the task
    pub fn configure_with<F>(&mut self, config: F) -> &mut Self
    where
        F: FnOnce(
                &mut T,
                &mut TaskOptions<T::ExecutableTask>,
                &Project,
            ) -> Result<(), ProjectError>
            + Send
            + Sync
            + 'static,
    {
        let mut lock = self.inner.write().unwrap();
        lock.configurations.push(TaskConfig2::new(config));
        drop(lock);
        self
    }

    // /// Get the output of a task. Only works if the property is created as an output (probably).
    // pub fn output<Ty : Property + Debug>(&self, id: &str) -> Delayed<Ty> {
    //     todo!()
    // }

    pub fn provider(&mut self) -> TaskProvider<T> {
        self.clone()
    }
}

impl<T: Task, F> TaskConfigurator<T> for F
where
    for<'a> F: FnOnce(
        &'a mut T,
        &'a mut TaskOptions<T::ExecutableTask>,
        &'a Project,
    ) -> Result<(), ProjectError>,
    F: Send + Sync + 'static,
{
    fn configure_task(
        self,
        task: &mut T,
        opts: &mut TaskOptions<T::ExecutableTask>,
        project: &Project,
    ) -> Result<(), ProjectError> {
        (self)(task, opts, project)
    }
}

pub trait TaskConfigurator<T: Task>: 'static + Send + Sync {
    fn configure_task(
        self,
        task: &mut T,
        opts: &mut TaskOptions<T::ExecutableTask>,
        project: &Project,
    ) -> Result<(), ProjectError>;
}
assert_obj_safe!(TaskConfigurator<crate::task::Empty>);

struct TaskProviderInner<T: Task> {
    id: TaskId,
    c_pointer: Weak<RwLock<TaskContainerInner<T::ExecutableTask>>>,
    configurations: Vec<TaskConfig2<T>>,
}

trait ResolveTask<T: Executable> {
    fn resolve_task(&self, project: &Project) -> Result<T, ProjectError>;
}

impl<T: Task + 'static> ResolveTask<T::ExecutableTask> for Arc<RwLock<TaskProviderInner<T>>> {
    fn resolve_task(&self, project: &Project) -> Result<T::ExecutableTask, ProjectError> {
        let (task, options) =(|| -> Result<_, ProjectError> {
            let mut inner = self.write().unwrap();
            let mut task = T::create_task(inner.id.clone(), project);
            let mut options = TaskOptions::default();
            let configurations = std::mem::replace(&mut inner.configurations, vec![]);
            for configurator in configurations {
                configurator.configure(&mut task, &mut options, project)?;
            }
            Ok((task, options))
        })()?;
        let inner = self.read().unwrap();
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

pub struct TaskConfig2<T: Task> {
    func: Box<
        dyn FnOnce(
                &mut T,
                &mut TaskOptions<T::ExecutableTask>,
                &Project,
            ) -> Result<(), ProjectError>
            + Send
            + Sync
        ,
    >,
}

impl< T: Task> TaskConfig2<T> {
    pub fn new<F>(func: F) -> Self
    where
        F: Send + Sync + 'static,
        F: FnOnce(&mut T, &mut TaskOptions<T::ExecutableTask>, &Project) -> Result<(), ProjectError>,
    {
        let boxed = Box::new(func);
        Self {
            func: boxed
        }
    }

    pub fn configure(self, task: &mut T, opts: &mut TaskOptions<T::ExecutableTask>, project: &Project) -> Result<(), ProjectError> {
        (self.func)(task, opts, project)
    }
}

#[cfg(test)]
mod tests {
    use crate::file::RegularFile;
    use crate::file_collection::FileCollection;
    use crate::identifier::TaskId;
    use crate::task::task_container::TaskContainer;
    use crate::task::Empty;
    use std::any::Any;

    #[test]
    fn task_provider_as_buildable() {
        let mut task_container = TaskContainer::new();
        let task_provider = task_container.register_task::<Empty>(TaskId::new("task").unwrap());

        let mut file_collection = FileCollection::new();
        file_collection.built_by(&task_provider);
    }
}
