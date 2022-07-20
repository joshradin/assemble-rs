use crate::__export::TaskId;
use crate::project::buildable::Buildable;
use crate::project::{ProjectError, ProjectResult};
use crate::task::up_to_date::UpToDate;
use crate::task::{CreateTask, HasTaskId, InitializeTask};
use crate::{BuildResult, Executable, Project, Task};
use log::{debug, info, trace};
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use colored::Colorize;
use convert_case::Case;
use convert_case::Casing;

/// Get a list of tasks within this project.
#[derive(Debug, Default)]
pub struct TaskReport;

impl UpToDate for TaskReport {}

impl InitializeTask for TaskReport {}

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

            group_to_tasks.entry(group)
                .or_default()
                .push(format!("{}{}", id.to_string().green().bold(), description));
        }

        let last = group_to_tasks.remove("");

        let match_group = |group: &String| {
            if let Some(Some(request)) = project.get_property("tasks.group") {
                group.to_lowercase() == request.to_lowercase()
            } else {
                true
            }
        };

        for (group, task_info) in group_to_tasks {
            if !match_group(&group) {
                continue;
            }
            info!("{}", format!("{} tasks:", group.to_case(Case::Title)).underline());
            for task_info in task_info {
                info!("  {}", task_info);
            }
            info!("");
        }

        if project.has_property("tasks.all") {
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