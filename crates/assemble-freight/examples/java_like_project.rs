use assemble_core::defaults::tasks::Empty;
use assemble_core::project::{ProjectResult, SharedProject};
use assemble_core::{execute_assemble, Project};
use assemble_freight::utils::FreightError;
use assemble_freight::{freight_main, FreightArgs};
use clap::Parser;
use std::process::exit;

fn main() {
    if execute_assemble::<(), FreightError, _>(|| {
        let project = Project::temp(None);

        project.tasks().register_task::<Empty>("clean")?;
        let process_resources = project
            .tasks()
            .register_task::<Empty>("process_resources")?;
        let compile_java = project.tasks().register_task::<Empty>("compile_java")?;
        let mut classes = project.tasks().register_task::<Empty>("classes")?;
        classes.configure_with(|classes, _| {
            classes.depends_on(compile_java);
            classes.depends_on("process_resources");
            classes.do_first(|_, _| {
                println!("running lifecycle task classes");
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

        let args = FreightArgs::parse();
        freight_main(&project, args)?;

        Ok(())
    })
    .is_none()
    {
        exit(101)
    }
}
