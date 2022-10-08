use assemble_core::defaults::tasks::Empty;
use assemble_core::exception::BuildException;
use assemble_core::logging::LOGGING_CONTROL;
use assemble_core::task::HasTaskId;
use assemble_core::Project;
use assemble_freight::cli::FreightArgs;
use assemble_freight::ops::execute_tasks;
use assemble_freight::utils::FreightError;
use clap::Parser;
use log::{error, info};

use std::thread;
use std::time::Duration;

fn main() -> Result<(), FreightError> {
    let args: FreightArgs = FreightArgs::parse();
    let handle = args.logging().init_root_logger().ok().flatten();
    let project = {
        let project = Project::temp(None);
        let max_workers: usize = num_cpus::get() / 2;
        project.with_mut(|project| {
            let mut worker_tasks = vec![];
            for i in 0..(max_workers * 4) {
                worker_tasks.push(
                    project
                        .task_container_mut()
                        .register_task_with::<Empty, _>(
                            &format!("minorTask{}", i + 1),
                            move |t, _| {
                                t.do_first(move |t, _p| {
                                    thread::sleep(Duration::from_millis((i as u64 * 100) % 4000));
                                    info!("finished task {}", t.task_id());
                                    Ok(())
                                })
                            },
                        )
                        .unwrap(),
                );
            }

            project
                .task_container_mut()
                .register_task_with::<Empty, _>("joinTask", |task, _| {
                    for c_task in worker_tasks {
                        task.depends_on(c_task);
                    }
                    task.do_first(|t, _| {
                        info!("finished task {}", t.task_id());
                        Ok(())
                    })?;
                    Ok(())
                })
                .unwrap();
        });
        project
    };

    let results = execute_tasks(&project, &args)?;

    for result in results {
        match result.result.as_ref().map_err(|e| e.kind()) {
            Err(BuildException::Error(error)) => {
                error!("task {} failed", result.id);
                error!("reason: {}", error);
            }
            _ => {}
        }
    }

    if let Some(handle) = handle {
        LOGGING_CONTROL.stop_logging();
        handle.join().unwrap();
    }

    Ok(())
}
