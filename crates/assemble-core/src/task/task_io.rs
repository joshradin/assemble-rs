use crate::__export::ProjectResult;
use crate::task::work_handler::output::Output;
use crate::{Executable, Task};

pub mod work;

/// Configures the inputs and outputs of a task
pub trait TaskIO<T: Task = Self> {
    /// During the initialization of the task, configures the inputs and outputs of the task.
    fn configure_io(_task: &mut Executable<T>) -> ProjectResult {
        Ok(())
    }

    /// Recovers outputs from previous run if up-to-date
    fn recover_outputs(&mut self, _output: &Output) -> ProjectResult {
        Ok(())
    }
}
