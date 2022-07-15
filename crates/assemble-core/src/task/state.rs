use crate::__export::TaskProperties;
use crate::identifier::TaskId;
use crate::properties::{FromProperties, Provides};
use crate::task::SaveTaskState;
use crate::Task;
use once_cell::sync::OnceCell;
use std::any;
use std::any::{type_name, Any, TypeId};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::marker::PhantomData;
use std::sync::{Arc, LockResult, PoisonError, RwLock, RwLockReadGuard, Weak};

#[derive(Debug, thiserror::Error)]
pub enum TaskStateError {
    #[error("TaskStateConnection {0} Poisoned")]
    PoisonError(TaskId),
    #[error("Task with id {task} exists, but is not of type {given}")]
    IncorrectTaskType { task: TaskId, given: &'static str },
    #[error("Task {0} not registered")]
    TaskNotRegistered(TaskId),
}

pub type TaskStateResult<T> = Result<T, TaskStateError>;

/// Save and get data about task state.
///
/// Should be used to create providers for tasks
pub struct TaskStateContainer {
    map: HashMap<TaskId, Arc<RwLock<TaskStateConnection>>>,
}

impl TaskStateContainer {
    pub fn new() -> Self {
        Self {
            map: Default::default(),
        }
    }

    pub fn register_task(&mut self, task: &TaskId) {
        match self.map.entry(task.clone()) {
            Entry::Occupied(_) => {}
            Entry::Vacant(v) => {
                v.insert(Arc::new(RwLock::new(TaskStateConnection::new(
                    task.clone(),
                ))));
            }
        }
    }

    pub fn insert<T: Task + 'static>(&mut self, key: &TaskId, task: T) -> TaskStateResult<()> {
        let mut connection = self
            .map
            .entry(key.clone())
            .or_insert(Arc::new(RwLock::new(TaskStateConnection::new(key.clone()))));
        let mut guard = connection
            .write()
            .map_err(|_| TaskStateError::PoisonError(key.clone()))?;
        guard.insert(task)?;
        Ok(())
    }

    fn get_connection<T: Task + 'static>(
        &mut self,
        id: &TaskId,
    ) -> TaskStateResult<&Arc<RwLock<TaskStateConnection>>> {
        let connection = self
            .map
            .get(id)
            .ok_or_else(|| TaskStateError::TaskNotRegistered(id.clone()))?;

        let connection_guard: RwLockReadGuard<TaskStateConnection> = connection
            .read()
            .map_err(|_| TaskStateError::PoisonError(id.clone()))?;
        if connection_guard.is_type::<T>() {
            Ok(connection)
        } else {
            Err(TaskStateError::IncorrectTaskType {
                task: id.clone(),
                given: std::any::type_name::<T>(),
            })
        }
    }

    pub fn get<T: Task + FromProperties + 'static>(
        &mut self,
        id: &TaskId,
    ) -> TaskStateResult<TaskStateProvider<T>> {
        self.get_connection::<T>(id)
            .map(|arc| TaskStateProvider::new(id.clone(), arc))
    }

    pub fn get_with<T, R, F>(
        &mut self,
        id: &TaskId,
        func: F,
    ) -> TaskStateResult<TaskStateProvider<T, R, F>>
    where
        T: Task + 'static,
        R: Send + Sync + Clone,
        F: Fn(&T) -> R + Send + Sync,
    {
        self.get_connection::<T>(id)
            .map(|arc| TaskStateProvider::with(id.clone(), arc, func))
    }
}

struct TaskStateConnection {
    id: TaskId,
    type_id: OnceCell<TypeId>,
    task_props: TaskProperties,
}

impl TaskStateConnection {
    /// Check if this connection is of type `T`.
    ///
    /// Returns false is type id is not set.
    pub fn is_type<T: Any + Send + Sync>(&self) -> bool {
        self.type_id
            .get()
            .map(|id| id == &TypeId::of::<T>())
            .unwrap_or(false)
    }
}

impl TaskStateConnection {
    pub fn new(id: TaskId) -> Self {
        Self {
            id,
            type_id: Default::default(),
            task_props: Default::default(),
        }
    }

    pub fn insert<T: Task + 'static>(&mut self, task: T) -> Result<(), TaskStateError> {
        let id = task.type_id();
        let type_id = self.type_id.get_or_init(|| id);
        if type_id != &id {
            Err(TaskStateError::IncorrectTaskType {
                task: self.id.clone(),
                given: type_name::<T>(),
            })
        } else {
            task.set_properties(&mut self.task_props);
            Ok(())
        }
    }
}

pub struct TaskStateProvider<T: Task, R = T, F: Fn(&T) -> R = fn(&T) -> T> {
    id: TaskId,
    connection: Weak<RwLock<TaskStateConnection>>,
    func: F,
    _task_type: PhantomData<(T, R)>,
}

impl<T: Task, R, F: Fn(&T) -> R + Clone> Clone for TaskStateProvider<T, R, F> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            connection: self.connection.clone(),
            func: self.func.clone(),
            _task_type: PhantomData,
        }
    }
}

impl<T: Task, R: Send + Sync, F: Fn(&T) -> R + Send + Sync> TaskStateProvider<T, R, F> {
    fn with(id: TaskId, connection: &Arc<RwLock<TaskStateConnection>>, func: F) -> Self {
        Self {
            id,
            connection: Arc::downgrade(connection),
            func,
            _task_type: Default::default(),
        }
    }

    pub fn id(&self) -> &TaskId {
        &self.id
    }
}

impl<T: Task + FromProperties> TaskStateProvider<T> {
    fn new(id: TaskId, connection: &Arc<RwLock<TaskStateConnection>>) -> Self {
        Self::with(id, connection, |task: &T| {
            let mut target = T::new_default_task();
            task.save_state(&mut target);
            target
        })
    }
}

impl<T: Task, R, F: Fn(&T) -> R> Debug for TaskStateProvider<T, R, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} Provider", self.id)
    }
}

impl<T: Task, R, F: Fn(&T) -> R> Display for TaskStateProvider<T, R, F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} Provider", self.id)
    }
}

impl<T: Task + FromProperties + 'static, R: Send + Sync + Clone, F: Fn(&T) -> R + Send + Sync>
    Provides<R> for TaskStateProvider<T, R, F>
{
    fn missing_message(&self) -> String {
        format!("No task state could be determined for {}", self.id)
    }

    fn try_get(&self) -> Option<R> {
        let upgraded = self.connection.upgrade()?;
        let mut guard = upgraded.try_write().ok()?;
        let task: T = FromProperties::from_properties_projectless(&mut guard.task_props);
        Some((self.func)(&task))
    }
}
