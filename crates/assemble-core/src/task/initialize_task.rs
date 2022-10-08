use crate::__export::ProjectResult;
use crate::{Executable, Project, Task};

/// Trait to implement to initialize a task after it's been wrapped in an Executable
pub trait InitializeTask<T: Task = Self> {
    /// Initialize tasks
    fn initialize(_task: &mut Executable<T>, _project: &Project) -> ProjectResult {
        Ok(())
    }
}
