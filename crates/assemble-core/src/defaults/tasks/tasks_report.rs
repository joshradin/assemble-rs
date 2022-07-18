use crate::__export::TaskId;
use crate::project::buildable::Buildable;
use crate::project::{ProjectError, ProjectResult};
use crate::task::{CreateTask, HasTaskId, InitializeTask};
use crate::{BuildResult, Executable, Project, Task};
use log::{debug, info, trace};
use std::collections::HashSet;

/// Get a list of tasks within this project.
#[derive(Debug, Default)]
pub struct TaskReport;

impl Task for TaskReport {
    fn task_action(task: &mut Executable<Self>, project: &Project) -> BuildResult {
        let container = project.task_container();
        let tasks = container
            .get_tasks()
            .into_iter()
            .cloned()
            .collect::<Vec<TaskId>>();
        for task_id in tasks {
            let mut handle = container.get_task(&task_id)?;
            debug!("got task handle {:?}", handle);

            if handle.task_id() == task.task_id() {
                trace!("skipping because its this task and self-referential tasks cause cycles");
                continue;
            }

            let full_task = handle.resolve(project)?;

            let id = full_task.task_id();
            info!("- {}", id);
        }
        Ok(())
    }
}
