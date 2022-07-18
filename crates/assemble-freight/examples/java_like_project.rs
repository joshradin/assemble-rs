use std::fmt::{Debug, Formatter};
use assemble_core::defaults::tasks::Empty;
use assemble_core::project::{ProjectResult, SharedProject};
use assemble_core::{BuildResult, Executable, execute_assemble, Project, Task};
use assemble_freight::utils::FreightError;
use assemble_freight::{freight_main, FreightArgs};
use clap::Parser;
use std::process::exit;
use assemble_core::__export::{CreateTask, InitializeTask, TaskId};
use assemble_core::properties::{Prop, Provides};
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::CreateTask;

#[derive(CreateTask, Debug)]
struct PrintString {
    string: Prop<String>
}

impl UpToDate for PrintString {}

impl InitializeTask for PrintString {
    fn initialize(task: &mut Executable<Self>, _project: &Project) -> ProjectResult {
        let prop = task.string.clone();
        task.work()
            .add_prop(&prop)
    }
}


impl Task for PrintString {
    fn task_action(task: &mut Executable<Self>, _project: &Project) -> BuildResult {
        println!("{}", task.string.get());
        Ok(())
    }
}

fn main() {
    if execute_assemble::<(), FreightError, _>(|| {
        let project = Project::with_id("java_like")?;

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

        let mut print_string = project.tasks().register_task::<PrintString>("hello_world")?;
        print_string.configure_with(|pr, _| { pr.string.set("Hello, World"); Ok(())})?;

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
