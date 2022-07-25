use crate::__export::{CreateTask, TaskId};
use crate::project::ProjectResult;
use crate::task::flags::{OptionDeclarationBuilder, OptionDeclarations, OptionsDecoder};
use crate::task::up_to_date::UpToDate;
use crate::task::InitializeTask;
use crate::{BuildResult, Executable, Project, Task};
use log::info;
use std::fmt::Write;
use colored::Colorize;
use crate::text_factory::{AssembleFormatter, less_important_string};
use crate::text_factory::list::TextListFactory;


/// The help task. Defines help for the, well, task
#[derive(Debug)]
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

impl Task for Help {
    fn task_action(task: &mut Executable<Self>, project: &Project) -> BuildResult {
        if let Some(task_request) = &task.task_request {
            Ok(())
        } else {
            let mut text_factory = AssembleFormatter::default();

            writeln!(text_factory.important(), "* Welcome to the assemble builder for {}", project.id())?;
            writeln!(text_factory, "")?;
            writeln!(text_factory, "To find out what tasks are available for this project, run {}", ":tasks".bold())?;
            writeln!(text_factory, "")?;

            writeln!(text_factory.important(), "* To display more logging information:")?;

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
