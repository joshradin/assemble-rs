use assemble_core::defaults::tasks::Empty;
use assemble_core::exception::BuildException;

use assemble_core::logging::LOGGING_CONTROL;

use assemble_core::task::ExecutableTask;
use assemble_core::{execute_assemble, Project, Task};
use assemble_freight::cli::FreightArgs;
use assemble_freight::ops::execute_tasks;
use assemble_freight::utils::FreightError;
use clap::Parser;
use log::info;

use assemble_core::error::PayloadError;
use std::process::exit;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    if execute_assemble::<(), PayloadError<FreightError>, _>(|| {
        let args: FreightArgs = FreightArgs::parse();
        let handle = args.logging().init_root_logger().ok().flatten();

        let project = Project::with_id("java_like").map_err(PayloadError::into)?;

        project.with_mut(|project| {
            let properties = args.properties().properties();
            for (key, value) in properties {
                project.set_property(key, value);
            }
        });

        project.tasks().register_task::<Empty>("clean").map_err(PayloadError::into)?;
        let _process_resources = project
            .tasks()
            .register_task::<Empty>("process_resources").map_err(PayloadError::into)?;
        let compile_java = project.tasks().register_task::<Empty>("compile_java").map_err(PayloadError::into)?;
        let mut classes = project.tasks().register_task::<Empty>("classes").map_err(PayloadError::into)?;
        classes.configure_with(|classes, _| {
            classes.depends_on(compile_java);
            classes.depends_on("process_resources");
            classes.set_description("lifecycle task to create all classes in main source set\nCalls the compile task and process resources task for the source set");
            classes.set_group("build");
            classes.do_first(|_, _| {
                info!("running lifecycle task classes");
                sleep(Duration::from_millis(3000));
                Ok(())
            })?;
            classes.do_last(|e, _| {
                info!("did_work: {}", e.work().did_work());
                Ok(())
            })?;
            Ok(())
        }).map_err(PayloadError::into)?;


        let _assemble =
            project
                .tasks()
                .register_task_with::<Empty, _>("assemble", |assemble, _| {
                    assemble.depends_on(classes);
                    Ok(())
                }).map_err(PayloadError::into)?;

        project
                .tasks()
                .register_task_with::<Empty, _>("check", |check, _| {
                    check.set_group("verification");
                    check.set_description("lifecycle task to run verifications on the project");
                    check.depends_on("test");
                    Ok(())
                }).map_err(PayloadError::into)?;

        project
            .tasks()
            .register_task_with::<Empty, _>("test", |test, _| {
                test.set_group("verification");
                test.set_description("Runs tests");
                test.depends_on("classes");
                Ok(())
            }).map_err(PayloadError::into)?;

        project
            .tasks()
            .register_task_with::<Empty, _>("build", |build, _| {
                build.set_group("package");
                build.set_description("Assembles and verifies this project");
                build.depends_on("check");
                build.depends_on("assemble");
                Ok(())
            }).map_err(PayloadError::into)?;


        let results = execute_tasks(&project, &args)?;

        for result in results {
            match result.result.as_ref().map_err(|e| e.kind()) {
                Err(BuildException::Error(error)) => {
                    info!("task {} failed", result.id);
                    info!("reason: {}", error);
                }
                _ => {

                }
            }
        }

        if let Some(handle) = handle {
            LOGGING_CONTROL.stop_logging();
            handle.join().unwrap();
        }

        Ok(())
    })
    .is_none()
    {
        exit(101)
    }
}
