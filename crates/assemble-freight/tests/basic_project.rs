use assemble_core::Project;
use assemble_core::task::{Empty, TaskOptions};
use assemble_freight::{freight_main, FreightArgs};
use assemble_freight::utils::FreightError;

#[test]
fn resolve_and_execute_project() -> Result<(), FreightError>{

    let project = {
        let mut project = Project::default();

        project.task::<Empty>("task1")?.configure(|t, opts, _| {
            opts.depend_on("task2");
            println!("running task 1");
            Ok(())
        });
        project.task::<Empty>("task2")?.configure(|t, opts, _| {
            opts.depend_on("task3");
            println!("running task 2");
            Ok(())
        });
        project.task::<Empty>("task3")?.configure(|t, opts, _| {
            println!("running task 3");
            Ok(())
        });

        project
    };

    let freight_args = FreightArgs::command_line("task1 task2 task3 --debug");

    let results = freight_main(
        project, freight_args
    )?;

    assert_eq!(results.len(), 3, "all tasks should run");
    assert!(results.iter().all(|r| r.result.is_ok()));
    Ok(())
}