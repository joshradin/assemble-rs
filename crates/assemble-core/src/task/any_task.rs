use crate::__export::TaskId;

use crate::project::error::ProjectResult;
use crate::project::SharedProject;
use crate::task::{
    BuildableTask, FullTask, HasTaskId, ResolveExecutable, TaskHandle, TaskOrdering,
};
use crate::{Project, Task};
use std::any::{Any, TypeId};
use std::fmt::{Debug, Formatter};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AnyTaskHandle {
    id: TaskId,
    handle: Arc<Mutex<AnyTaskHandleInner>>,
}

impl Debug for AnyTaskHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "AnyTaskHandle {:?}", self.id)
    }
}

impl HasTaskId for AnyTaskHandle {
    fn task_id(&self) -> TaskId {
        self.id.clone()
    }
}

impl BuildableTask for AnyTaskHandle {
    fn ordering(&self) -> Vec<TaskOrdering> {
        self.with_inner(|inner| inner.buildable().ordering())
    }
}

impl AnyTaskHandle {
    pub fn new<T: Task + Send + Sync + 'static>(provider: TaskHandle<T>) -> Self {
        Self {
            id: provider.task_id().clone(),
            handle: Arc::new(Mutex::new(AnyTaskHandleInner::new(provider))),
        }
    }

    fn with_inner<R, F: FnOnce(&mut AnyTaskHandleInner) -> R>(&self, func: F) -> R {
        let mut guard = self.handle.lock().expect("couldn't get handle");
        (func)(&mut *guard)
    }

    pub fn is<T: Task + Send+ Sync + 'static>(&self) -> bool {
        self.with_inner(|handle| handle.is::<T>())
    }

    pub fn as_type<T: Task + Send + Sync + 'static>(&self) -> Option<TaskHandle<T>> {
        if !self.is::<T>() {
            return None;
        }
        self.with_inner(|handle| handle.as_type::<T>())
    }

    fn executable(&mut self, project: &SharedProject) -> ProjectResult<Box<dyn FullTask>> {
        self.with_inner(|p| p.resolvable().get_executable(project))
    }

    pub fn resolve(&mut self, project: &Project) -> ProjectResult<Box<dyn FullTask>> {
        self.executable(&project.as_shared())
    }

    pub fn resolve_shared(&mut self, project: &SharedProject) -> ProjectResult<Box<dyn FullTask>> {
        self.executable(project)
    }
}

struct AnyTaskHandleInner {
    task_type: TypeId,
    as_buildable: Box<dyn BuildableTask + Send>,
    as_resolvable: Box<dyn ResolveExecutable + Send>,
    as_any: Box<dyn Any + Send>,
}

impl Debug for AnyTaskHandleInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TaskConnection {{ ... }}")
    }
}

impl AnyTaskHandleInner {
    fn new<T: Task + Send  + Sync+ 'static>(provider: TaskHandle<T>) -> Self {
        let task_type = TypeId::of::<T>();
        let as_buildable: Box<dyn BuildableTask + Send> = Box::new(provider.clone());
        let as_resolvable: Box<dyn ResolveExecutable + Send> = Box::new(provider.clone());
        let as_any: Box<dyn Any + Send> = Box::new(provider);
        Self {
            task_type,
            as_buildable,
            as_resolvable,
            as_any,
        }
    }

    fn is<T: Task + Send + Sync + 'static>(&self) -> bool {
        self.task_type == TypeId::of::<T>()
    }

    fn as_type<T: Task + Send + Sync + 'static>(&self) -> Option<TaskHandle<T>> {
        if !self.is::<T>() {
            return None;
        }
        self.as_any.downcast_ref::<TaskHandle<T>>().cloned()
    }

    fn buildable(&self) -> &dyn BuildableTask {
        self.as_buildable.as_ref()
    }

    fn resolvable(&mut self) -> &mut dyn ResolveExecutable {
        self.as_resolvable.as_mut()
    }

}

assert_impl_all!(AnyTaskHandleInner: Send);
