use crate::__export::TaskId;
use crate::exception::BuildException;
use crate::project::error::ProjectResult;
use crate::task::create_task::CreateTask;
use crate::task::flags::{OptionDeclarationBuilder, OptionDeclarations, OptionsDecoder};
use crate::task::initialize_task::InitializeTask;
use crate::task::task_io::TaskIO;
use crate::task::up_to_date::UpToDate;
use crate::unstable::text_factory::{
    less_important_string, list::TextListFactory, AssembleFormatter,
};

use crate::{BuildResult, Executable, Project, Task};
use colored::Colorize;
use log::info;
use std::fmt::Write;

/// The help task. Defines help for the, well, task
#[derive(Debug)]
pub struct Help {
    task_request: Option<String>,
}

impl UpToDate for Help {}

impl InitializeTask for Help {}

impl CreateTask for Help {
    fn new(_using_id: &TaskId, _project: &Project) -> ProjectResult<Self> {
        Ok(Self { task_request: None })
    }

    fn description() -> String {
        "Print help information for the project using a specific task".to_string()
    }

    fn options_declarations() -> Option<OptionDeclarations> {
        Some(OptionDeclarations::new::<Help, _>([
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

impl TaskIO for Help {}

impl Task for Help {
    fn task_action(task: &mut Executable<Self>, project: &Project) -> BuildResult {
        if let Some(_task_request) = &task.task_request {
            Err(BuildException::custom("help for task requests not implemented").into())
        } else {
            let mut text_factory = AssembleFormatter::default();

            writeln!(
                text_factory.important(),
                "* Welcome to the assemble builder for {}",
                project.id()
            )?;
            writeln!(text_factory)?;
            writeln!(
                text_factory,
                "To find out what tasks are available for this project, run {}",
                ":tasks".bold()
            )?;
            writeln!(text_factory)?;

            writeln!(
                text_factory.important(),
                "* To display more logging information:"
            )?;

            let list = TextListFactory::new(less_important_string("> ".yellow()))
                .element("For more detail, run with --debug")
                .element("For an overwhelming amount of data, run with --trace")
                .finish();

            write!(text_factory, "{}", list)?;

            info!("{}", text_factory);
            info!("");

            Ok(())
        }
    }
}
