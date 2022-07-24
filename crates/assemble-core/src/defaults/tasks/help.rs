use log::info;
use crate::__export::{CreateTask, TaskId};
use crate::project::ProjectResult;
use crate::task::flags::{OptionDeclarationBuilder, OptionDeclarations, OptionsDecoder};
use crate::task::up_to_date::UpToDate;
use crate::task::InitializeTask;
use crate::{BuildResult, Executable, Project, Task};

/// The help task. Defines help for the, well, task
#[derive(Debug, CreateTask)]
pub struct Help {
    task_request: Option<String>,
}

impl UpToDate for Help {}

impl InitializeTask for Help {}

impl CreateTask for Help {
    fn new(using_id: &TaskId, project: &Project) -> ProjectResult<Self> {
        Ok(Self { task_request: None })
    }

    fn description() -> String {
        "Print help information for the project using a specific task".to_string()
    }

    fn options_declarations() -> Option<OptionDeclarations> {
        Some(OptionDeclarations::new([
            OptionDeclarationBuilder::<String>::new("task")
                .optional(true)
                .use_from_str()
                .build(),
        ]))
    }

    fn try_set_from_decoder(&mut self, decoder: &OptionsDecoder) -> ProjectResult<()> {
        self.task_request = decoder.get_value::<String>("task")?;
        Ok(())
    }
}

impl Task for Help {
    fn task_action(task: &mut Executable<Self>, project: &Project) -> BuildResult {
        if let Some(task_request) = &task.task_request {
            Ok(())
        } else {

        }
    }
}
