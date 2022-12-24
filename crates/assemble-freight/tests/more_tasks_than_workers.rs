use assemble_core::defaults::tasks::Empty;

use assemble_core::Project;
use assemble_freight::cli::FreightArgs;

use assemble_freight::ops::{execute_tasks, execute_tasks2};
use assemble_freight::utils::FreightError;

#[test]
#[ignore]
fn can_handle_more_tasks_than_workers() -> Result<(), FreightError> {
    let project = Project::temp(None);
    let max_workers: usize = num_cpus::get() / 2;
    project.with_mut(|project| {
        let mut worker_tasks = vec![];
        for i in 0..(max_workers * 4) {
            worker_tasks.push(
                project
                    .task_container_mut()
                    .register_task::<Empty>(&format!("minorTask{}", i))
                    .unwrap(),
            );
        }

        project
            .task_container_mut()
            .register_task_with::<Empty, _>("joinTask", |task, _| {
                for c_task in worker_tasks {
                    task.depends_on(c_task);
                }
                Ok(())
            })
            .unwrap();
    });

    let freight_args = FreightArgs::command_line(format!("joinTask --workers {max_workers}"));

    let results = execute_tasks(&project, &freight_args)?;

    println!("{:#?}", results);

    Ok(())
}
