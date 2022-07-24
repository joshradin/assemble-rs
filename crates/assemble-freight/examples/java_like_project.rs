use assemble_core::__export::{CreateTask, InitializeTask, TaskId};
use assemble_core::defaults::tasks::Empty;
use assemble_core::project::{ProjectResult, SharedProject};
use assemble_core::properties::{Prop, Provides};
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::task::ExecutableTask;
use assemble_core::{execute_assemble, BuildResult, Executable, Project, Task};
use assemble_freight::core::cli::FreightArgs;
use assemble_freight::ops::execute_tasks;
use assemble_freight::utils::FreightError;
use clap::Parser;
use std::fmt::{Debug, Formatter};
use std::process::exit;

fn main() {
    if execute_assemble::<(), FreightError, _>(|| {
        let args: FreightArgs = FreightArgs::parse();
        let project = Project::with_id("java_like")?;

        project.with_mut(|project| {
            let properties = args.properties.properties();
            for (key, value) in properties {
                project.set_property(key, value);
            }
        });

        project.tasks().register_task::<Empty>("clean")?;
        let process_resources = project
            .tasks()
            .register_task::<Empty>("process_resources")?;
        let compile_java = project.tasks().register_task::<Empty>("compile_java")?;
        let mut classes = project.tasks().register_task::<Empty>("classes")?;
        classes.configure_with(|classes, _| {
            classes.depends_on(compile_java);
            classes.depends_on("process_resources");
            classes.set_description("lifecycle task to create all classes in main source set\nCalls the compile task and process resources task for the source set");
            classes.set_group("build");
            classes.do_first(|_, _| {
                println!("running lifecycle task classes");
                Ok(())
            })?;
            classes.do_last(|e, _| {
                println!("did_work: {}", e.work().did_work());
                Ok(())
            })?;
            Ok(())
        })?;


        let assemble =
            project
                .tasks()
                .register_task_with::<Empty, _>("assemble", |assemble, _| {
                    assemble.depends_on(classes);
                    Ok(())
                })?;

        project
                .tasks()
                .register_task_with::<Empty, _>("check", |check, _| {
                    check.set_group("verification");
                    check.set_description("lifecycle task to run verifications on the project");
                    check.depends_on("test");
                    Ok(())
                })?;

        project
            .tasks()
            .register_task_with::<Empty, _>("test", |test, _| {
                test.set_group("verification");
                test.set_description("Runs tests");
                test.depends_on("classes");
                Ok(())
            })?;

        project
            .tasks()
            .register_task_with::<Empty, _>("build", |build, _| {
                build.set_group("package");
                build.set_description("Assembles and verifies this project");
                build.depends_on("check");
                build.depends_on("assemble");
                Ok(())
            })?;


        execute_tasks(&project, &args)?;

        Ok(())
    })
    .is_none()
    {
        exit(101)
    }
}
