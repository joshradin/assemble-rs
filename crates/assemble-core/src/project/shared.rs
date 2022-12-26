//! Contains the shared project

use crate::__export::{ProjectError, ProjectResult, TaskId};
use crate::dependencies::dependency_container::ConfigurationHandler;
use crate::dependencies::RegistryContainer;
use crate::error::PayloadError;
use crate::identifier::{ProjectId, TaskIdFactory};
use crate::plugins::PluginAware;
use crate::project::finder::{TaskFinder, TaskPath};
use crate::project::GetProjectId;
use crate::task::task_container::TaskContainer;
use crate::task::{AnyTaskHandle, TaskHandle};
use crate::{project, Plugin, Project, Task, Workspace};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Weak};

/// The shared project allows for many projects to share references to the same
/// [`Project`](Project) instance.
#[derive(Debug, Clone)]
pub struct SharedProject(Arc<(TrueSharableProject, ProjectId)>);

pub(crate) type TrueSharableProject = RwLock<Project>;

impl SharedProject {
    pub(super) fn new_cyclic<F: FnOnce(&WeakSharedProject) -> Project>(func: F) -> Self {
        let modified = |weak: &Weak<(TrueSharableProject, ProjectId)>| {
            let weakened = WeakSharedProject(weak.clone());
            let project = func(&weakened);
            let id = project.id().clone();
            (RwLock::new(project), id)
        };
        let arc = Arc::new_cyclic(modified);
        Self(arc)
    }

    fn new(project: Project) -> Self {
        let id = project.id().clone();
        Self(Arc::new((RwLock::new(project), id)))
    }

    pub(crate) fn weak(&self) -> WeakSharedProject {
        WeakSharedProject(Arc::downgrade(&self.0))
    }

    pub fn with<F, R>(&self, func: F) -> R
    where
        F: FnOnce(&Project) -> R,
    {
        let guard = self
            .0
             .0
            .try_read()
            .unwrap_or_else(|| panic!("Couldn't get read access to {}", self));
        let r = (func)(&*guard);
        r
    }

    pub fn with_mut<F, R>(&self, func: F) -> R
    where
        F: FnOnce(&mut Project) -> R,
    {
        let mut guard = self
            .0
             .0
            .try_write()
            .unwrap_or_else(|| panic!("Couldn't get read access to {}", self));
        let r = (func)(&mut *guard);
        r
    }

    pub fn guard<'g, T, F: Fn(&Project) -> &T + 'g>(&'g self, func: F) -> ProjectResult<Guard<T>> {
        let guard = match self.0 .0.try_read() {
            Some(guard) => guard,
            None => {
                panic!("Accessing this immutable guard would block for {}", self)
            }
        };
        Ok(Guard::new(guard, func))
    }

    pub fn guard_mut<'g, T, F1, F2>(
        &'g self,
        ref_getter: F1,
        mut_getter: F2,
    ) -> ProjectResult<GuardMut<T>>
    where
        F1: Fn(&Project) -> &T + 'g,
        F2: Fn(&mut Project) -> &mut T + 'g,
    {
        let guard = match self.0 .0.try_write() {
            Some(guard) => guard,
            None => {
                panic!("Accessing this guard would block for {}", self)
            }
        };
        Ok(GuardMut::new(guard, ref_getter, mut_getter))
    }

    pub fn tasks(&self) -> GuardMut<TaskContainer> {
        self.guard_mut(
            |project| project.task_container(),
            |project| project.task_container_mut(),
        )
        .expect("couldn't safely get task container")
    }

    pub fn register_task<T: Task + Send + Sync + Debug + 'static>(
        &self,
        id: &str,
    ) -> ProjectResult<TaskHandle<T>> {
        self.tasks().with_mut(|t| t.register_task::<T>(id))
    }

    /// Gets a task with a given name
    pub fn get_task(&self, id: &TaskId) -> ProjectResult<AnyTaskHandle> {
        Ok(self.task_container().with(|t| {
            t.get_task(id)
                .cloned()
                .ok_or(ProjectError::IdentifierMissing(id.clone()))
        })?)
    }

    /// Gets a typed task
    pub fn get_typed_task<T: Task + Send + Sync>(
        &self,
        id: &TaskId,
    ) -> ProjectResult<TaskHandle<T>> {
        self.get_task(id).and_then(|id| {
            id.as_type::<T>()
                .ok_or(ProjectError::custom("invalid task type").into())
        })
    }

    /// Finds a task, using this project as the base task
    pub fn find_task<P: AsRef<TaskPath>>(&self, path: P) -> ProjectResult<AnyTaskHandle> {
        let finder = TaskFinder::new(self);
        let path = path.as_ref();
        let ids = finder
            .find(path)?
            .ok_or(ProjectError::TaskNotFound(path.to_owned()))?;

        match &ids[..] {
            [] => unreachable!(),
            [task_id] => self.get_task(task_id),
            [..] => Err(PayloadError::new(ProjectError::TooManyIdentifiersFound(
                ids,
                path.to_string(),
            ))),
        }
    }

    pub fn get_subproject<P>(&self, project: P) -> project::Result<SharedProject>
    where
        P: AsRef<str>,
    {
        self.with(|p| p.get_subproject(project).cloned())
    }

    /// Executes a callback on this project and all sub projects
    pub fn allprojects<F: Fn(&Project) + Clone>(&self, callback: F) {
        self.with(callback.clone());
        self.with(|s| {
            s.subprojects()
                .iter()
                .for_each(|&project| project.allprojects(callback.clone()))
        })
    }

    /// Executes a callback on this project and all sub projects
    pub fn allprojects_mut<F: Fn(&mut Project) + Clone>(&self, callback: F) {
        self.with_mut(callback.clone());
        self.with(|s| {
            s.subprojects()
                .iter()
                .for_each(|&project| project.allprojects_mut(callback.clone()))
        })
    }

    /// Gets the task container for this project
    pub fn task_container(&self) -> Guard<TaskContainer> {
        self.guard(|project| project.task_container())
            .expect("couldn't safely get task container")
    }

    pub(crate) fn task_id_factory(&self) -> Guard<TaskIdFactory> {
        self.guard(|project| project.task_id_factory())
            .expect("couldn't safely get task id factory")
    }

    /// Get access to the registries container
    pub fn registries<F, R>(&self, func: F) -> R
    where
        F: FnOnce(&mut RegistryContainer) -> R,
    {
        self.with(|r| {
            let mut registries_lock = r.registries.lock().unwrap();
            let registries = &mut *registries_lock;
            func(registries)
        })
    }

    /// Get a guard to the dependency container within the project
    pub fn configurations_mut(&mut self) -> GuardMut<ConfigurationHandler> {
        self.guard_mut(
            |project| project.configurations(),
            |project| project.configurations_mut(),
        )
        .expect("couldn't safely get dependencies container")
    }

    /// Get a guard to the dependency container within the project
    pub fn configurations(&self) -> Guard<ConfigurationHandler> {
        self.guard(|project| project.configurations())
            .expect("couldn't safely get dependencies container")
    }

    pub fn workspace(&self) -> Guard<Workspace> {
        self.guard(|p| &p.workspace)
            .expect("couldn't get workspace")
    }

    /// Apply a plugin to this.
    pub fn apply_plugin<P: Plugin<Project>>(&self) -> ProjectResult {
        self.with_mut(|p| p.apply_plugin::<P>())
    }
}

impl Display for SharedProject {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0 .1)
    }
}

impl Default for SharedProject {
    fn default() -> Self {
        Project::new().unwrap()
    }
}

impl TryFrom<Weak<(TrueSharableProject, ProjectId)>> for SharedProject {
    type Error = ();

    fn try_from(
        value: Weak<(TrueSharableProject, ProjectId)>,
    ) -> std::result::Result<Self, Self::Error> {
        let arc = value.upgrade().ok_or(())?;
        Ok(SharedProject(arc))
    }
}

impl TryFrom<WeakSharedProject> for SharedProject {
    type Error = PayloadError<ProjectError>;

    fn try_from(value: WeakSharedProject) -> std::result::Result<Self, Self::Error> {
        value.upgrade()
    }
}

impl From<Arc<(TrueSharableProject, ProjectId)>> for SharedProject {
    fn from(arc: Arc<(TrueSharableProject, ProjectId)>) -> Self {
        Self(arc)
    }
}

/// Provides a shortcut around the project
pub struct Guard<'g, T> {
    guard: RwLockReadGuard<'g, Project>,
    getter: Box<dyn Fn(&Project) -> &T + 'g>,
}

impl<'g, T> Guard<'g, T> {
    pub fn new<F>(guard: RwLockReadGuard<'g, Project>, getter: F) -> Self
    where
        F: Fn(&Project) -> &T + 'g,
    {
        Self {
            guard,
            getter: Box::new(getter),
        }
    }

    pub fn with<R, F: FnOnce(&T) -> R>(&self, func: F) -> R {
        let guard = &*self.guard;
        let t = (self.getter)(guard);
        let r = (func)(t);
        r
    }
}

impl<T> Deref for Guard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let guard = &*self.guard;
        (self.getter)(guard)
    }
}

/// Provides a shortcut around the project
pub struct GuardMut<'g, T> {
    guard: RwLockWriteGuard<'g, Project>,
    ref_getter: Box<dyn Fn(&Project) -> &T + 'g>,
    mut_getter: Box<dyn Fn(&mut Project) -> &mut T + 'g>,
}

impl<'g, T> GuardMut<'g, T> {
    pub fn new<F1, F2>(guard: RwLockWriteGuard<'g, Project>, ref_getter: F1, mut_getter: F2) -> Self
    where
        F1: Fn(&Project) -> &T + 'g,
        F2: Fn(&mut Project) -> &mut T + 'g,
    {
        Self {
            guard,
            ref_getter: Box::new(ref_getter),
            mut_getter: Box::new(mut_getter),
        }
    }

    pub fn with<R, F: FnOnce(&T) -> R>(&self, func: F) -> R {
        let guard = &*self.guard;
        let t = (self.ref_getter)(guard);
        let r = (func)(t);
        r
    }

    pub fn with_mut<R, F: FnOnce(&mut T) -> R>(&mut self, func: F) -> R {
        let guard = &mut *self.guard;
        let t = (self.mut_getter)(guard);
        let r = (func)(t);
        r
    }
}

impl<T> Deref for GuardMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let guard = &*self.guard;
        (self.ref_getter)(guard)
    }
}

impl<T> DerefMut for GuardMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let ref mut guard = *self.guard;
        (self.mut_getter)(guard)
    }
}

impl GetProjectId for SharedProject {
    fn project_id(&self) -> ProjectId {
        self.with(|p| p.project_id())
    }

    fn parent_id(&self) -> Option<ProjectId> {
        self.with(GetProjectId::parent_id)
    }

    fn root_id(&self) -> ProjectId {
        self.with(GetProjectId::root_id)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct WeakSharedProject(Weak<(TrueSharableProject, ProjectId)>);

impl WeakSharedProject {
    /// Upgrades a weakly shared project
    pub fn upgrade(self) -> project::Result<SharedProject> {
        self.0
            .upgrade()
            .map(SharedProject)
            .ok_or(ProjectError::NoSharedProjectSet.into())
    }
}
