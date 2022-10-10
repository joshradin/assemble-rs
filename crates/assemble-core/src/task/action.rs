use crate::{BuildResult, Executable, Project, Task};
use std::any::type_name;
use std::fmt::{Debug, Formatter};

/// Represents some work that can be done by a task
pub trait TaskAction<T: Task>: Send {
    /// Executes the task action on some executable task along with it's owning
    /// project.
    fn execute(&self, task: &mut Executable<T>, project: &Project) -> BuildResult<()>;
}

impl<F, T> TaskAction<T> for F
where
    F: Fn(&mut Executable<T>, &Project) -> BuildResult,
    F: Send,
    T: Task,
{
    fn execute(&self, task: &mut Executable<T>, project: &Project) -> BuildResult<()> {
        (self)(task, project)
    }
}

impl<T: Task> TaskAction<T> for Action<T> {
    fn execute(&self, task: &mut Executable<T>, project: &Project) -> BuildResult<()> {
        (self.func)(task, project)
    }
}

assert_obj_safe!(TaskAction<crate::defaults::tasks::Empty>);

pub type DynamicTaskAction<T> = dyn Fn(&mut Executable<T>, &Project) -> BuildResult + Send + Sync;
/// A structure to generically own a task action over `'static` lifetime
pub struct Action<T: Task> {
    func: Box<DynamicTaskAction<T>>,
}

impl<T: Task> Debug for Action<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Action<{}>", type_name::<T>())
    }
}

impl<T: Task> Action<T> {
    /// Creates a new action from a function
    pub fn new<F>(func: F) -> Self
    where
        F: Fn(&mut Executable<T>, &Project) -> BuildResult + 'static,
        F: Send + Sync,
    {
        Self {
            func: Box::new(func),
        }
    }
}

