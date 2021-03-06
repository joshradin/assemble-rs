use crate::__export::TaskId;
use crate::defaults::tasks::Empty;
use crate::project::buildable::Buildable;
use crate::project::{ProjectError, ProjectResult};
use crate::task::flags::{OptionDeclarationBuilder, OptionDeclarations, OptionsDecoder};
use crate::task::task_container::FindTask;
use crate::task::up_to_date::UpToDate;
use crate::task::{CreateTask, HasTaskId, InitializeTask};
use crate::{BuildResult, Executable, Project, Task};
use colored::Colorize;
use convert_case::Case;
use convert_case::Casing;
use log::{debug, info, trace};
use std::collections::{HashMap, HashSet};
use std::ops::Deref;

/// Get a list of tasks within this project.
#[derive(Debug)]
pub struct TaskReport {
    all: bool,
    groups: Option<Vec<String>>,
}

impl UpToDate for TaskReport {}

impl InitializeTask for TaskReport {}

impl CreateTask for TaskReport {
    fn new(using_id: &TaskId, project: &Project) -> ProjectResult<Self> {
        Ok(TaskReport {
            all: false,
            groups: None,
        })
    }

    fn options_declarations() -> Option<OptionDeclarations> {
        Some(OptionDeclarations::new::<Empty, _>([
            OptionDeclarationBuilder::flag("all").build(),
            OptionDeclarationBuilder::<String>::new("group")
                .allow_multiple_values(true)
                .optional(true)
                .use_from_str()
                .build(),
        ]))
    }

    fn try_set_from_decoder(&mut self, decoder: &OptionsDecoder) -> ProjectResult<()> {
        self.all = decoder.flag_present("all")?;
        self.groups = decoder.get_values::<String>("group")?;
        Ok(())
    }
}

impl Task for TaskReport {
    fn task_action(task: &mut Executable<Self>, project: &Project) -> BuildResult {
        let container = project.task_container();
        let tasks = container
            .get_tasks()
            .into_iter()
            .cloned()
            .collect::<Vec<TaskId>>();

        let mut group_to_tasks: HashMap<String, Vec<String>> = HashMap::new();

        for task_id in tasks {
            let mut handle = container.get_task(&task_id)?;
            debug!("got task handle {:?}", handle);

            if handle.task_id() == task.task_id() {
                trace!("skipping because its this task and self-referential tasks cause cycles");
                continue;
            }

            let full_task = handle.resolve(project)?;

            let mut id = full_task.task_id().deref().clone();

            if Some(project.id().deref()) == id.parent() {
                id = id.this_id().clone();
            }

            let group = full_task.group().to_lowercase();
            let description = {
                let desc = full_task.description();
                if desc.is_empty() {
                    "".to_string()
                } else {
                    format!(" - {}", desc.lines().next().unwrap().yellow())
                }
            };

            group_to_tasks.entry(group).or_default().push(format!(
                "{}{}",
                id.to_string().green().bold(),
                description
            ));
        }

        let last = group_to_tasks.remove("");

        let match_group = |group: &String| {
            if let Some(groups) = &task.groups {
                groups.contains(group)
            } else {
                true
            }
        };

        for (group, task_info) in group_to_tasks {
            if !match_group(&group) {
                continue;
            }
            info!(
                "{}",
                format!("{} tasks:", group.to_case(Case::Title)).underline()
            );
            for task_info in task_info {
                info!("  {}", task_info);
            }
            info!("");
        }

        if task.all {
            if let Some(task_info) = last {
                info!("{}", "Other tasks:".underline());
                for task_info in task_info {
                    info!("  {}", task_info);
                }
                info!("");
            }
        }

        Ok(())
    }
}
