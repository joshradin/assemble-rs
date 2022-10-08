use crate::__export::{ProjectResult, TaskId};
use crate::task::flags::{OptionDeclarations, OptionsDecoder};
use crate::{Project, Task};

/// Create tasks using a project.
pub trait CreateTask: Sized {
    /// Creates a new task. The using_id is the id of the task that's being created.
    fn new(using_id: &TaskId, project: &Project) -> ProjectResult<Self>;

    /// The default description for a Task
    fn description() -> String {
        String::new()
    }

    /// Gets an optional flags for this task.
    ///
    /// By defaults return `None`
    fn options_declarations() -> Option<OptionDeclarations> {
        None
    }

    /// Try to get values from a decoder.
    ///
    /// By default does not do anything.
    fn try_set_from_decoder(&mut self, _decoder: &OptionsDecoder) -> ProjectResult<()> {
        Ok(())
    }
}

impl<T: Default + Task> CreateTask for T {
    fn new(_: &TaskId, _: &Project) -> ProjectResult<Self> {
        Ok(T::default())
    }
}
