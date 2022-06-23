use assemble_core::Project;
use assemble_core::task::{Empty, TaskOptions};
use assemble_freight::{freight_main, FreightArgs};
use assemble_freight::core::ConstructionError;
use assemble_freight::utils::{FreightError, FreightResult};

#[test]
fn resolve_and_execute_project() -> Result<(), FreightError>{

    let project = {
        let mut project = Project::default();

        project.task::<Empty>("task1")?.configure_with(|t, opts, _| {
            opts.depend_on("task2");
            println!("configuring task 1");
            Ok(())
        });
        project.task::<Empty>("task2")?.configure_with(|t, opts, _| {
            opts.depend_on("task3");
            println!("configuring task 2");
            Ok(())
        });
        project.task::<Empty>("task3")?.configure_with(|t, opts, _| {
            println!("configuring task 3");
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

#[test]
fn detect_task_cycles() -> Result<(), FreightError> {

    let project = {
        let mut project = Project::default();

        project.task::<Empty>("task1")?.configure_with(|t, opts, _| {
            opts.depend_on("task2");
            println!("configuring task 1");
            Ok(())
        });
        project.task::<Empty>("task2")?.configure_with(|t, opts, _| {
            opts.depend_on("task3");
            println!("configuring task 2");
            Ok(())
        });
        project.task::<Empty>("task3")?.configure_with(|t, opts, _| {
            println!("configuring task 3");
            opts.depend_on("task1");
            Ok(())
        });

        project
    };


    let freight_args = FreightArgs::command_line("task1 task2 task3 --debug");

    let result = freight_main(
        project, freight_args
    );

    match result {

        Err(FreightError::ConstructError(ConstructionError::CycleFound { cycle })) => {
            eprintln!("found cycle consisting of: {:?}", cycle);
            return Ok(())
        }
        _ => {
            panic!("Should be a cyclical construction error")
        }
    }
}