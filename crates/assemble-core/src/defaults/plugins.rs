use crate::defaults::tasks::{Help, TaskReport, WrapperTask};
use crate::dependencies::project_dependency::ProjectDependencyPlugin;
use crate::plugins::Plugin;
use crate::project::error::ProjectResult;

use crate::Project;

/// The base plugin is applied to every project and supplies only needed tasks.
///
/// # Provided Tasks
/// - `tasks`: lists the available tasks in this project
#[derive(Default)]
pub struct BasePlugin;

/// The name of the task that reports all tasks in a project.
pub const TASKS_REPORT_TASK_NAME: &str = "tasks";
/// The name of the task that provides help information for the project
pub const HELP_TASK_NAME: &str = "help";
/// The name of the task that can create a wrapper for running assemble projects. Only present in the
/// root project
pub const WRAPPER_TASK_NAME: &str = "wrapper";
/// The assemble group are tasks that are important for the operation of an assemble project
pub const ASSEMBLE_GROUP: &str = "assemble";

impl Plugin for BasePlugin {
    fn apply(&self, project: &mut Project) -> ProjectResult {
        project
            .task_container_mut()
            .register_task_with::<TaskReport, _>(TASKS_REPORT_TASK_NAME, |tasks, _| {
                tasks.set_group(ASSEMBLE_GROUP);
                Ok(())
            })?;
        let mut help = project
            .task_container_mut()
            .register_task::<Help>(HELP_TASK_NAME)?;
        help.configure_with(|task, _| {
            task.set_group(ASSEMBLE_GROUP);
            Ok(())
        })?;
        project.set_default_tasks([help.id().clone()]);

        if project.parent_project().is_none() {
            project
                .task_container_mut()
                .register_task_with::<WrapperTask, _>(WRAPPER_TASK_NAME, |task, _| {
                    task.set_group(ASSEMBLE_GROUP);
                    Ok(())
                })?;
        }

        project.apply_plugin::<ProjectDependencyPlugin>()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::defaults::plugins::TASKS_REPORT_TASK_NAME;
    use crate::defaults::tasks::TaskReport;
    use crate::Project;

    #[test]
    fn base_always_applied() {
        let project = Project::temp(None);
        let handle = project.get_task(TASKS_REPORT_TASK_NAME);
        assert!(
            handle.is_ok(),
            "{} was not added to project",
            TASKS_REPORT_TASK_NAME
        );
        let handle = handle.unwrap();
        let task_report = handle.as_type::<TaskReport>();
        assert!(
            task_report.is_some(),
            "could not get {} as TaskReport",
            TASKS_REPORT_TASK_NAME
        );
    }
}
