use assemble_core::defaults::tasks::Empty;
use assemble_core::identifier::ProjectId;
use assemble_core::Project;
use assemble_freight::core::cli::FreightArgs;
use assemble_freight::core::ConstructionError;
use assemble_freight::ops::execute_tasks;
use assemble_freight::utils::{FreightError, FreightResult};

#[test]
fn resolve_and_execute_project() -> Result<(), FreightError> {
    let project_id = ProjectId::new("test")?;
    let project = {
        let mut project = Project::with_id(project_id.clone())?;

        project
            .tasks()
            .register_task::<Empty>("task3")?
            .configure_with(|t, opts| {
                println!("configuring task 3");
                Ok(())
            })?;
        project
            .tasks()
            .register_task::<Empty>("task2")?
            .configure_with(|t, opts| {
                t.depends_on("task3");
                println!("configuring task 2");
                Ok(())
            })?;
        project
            .tasks()
            .register_task::<Empty>("task1")?
            .configure_with(|t, opts| {
                t.depends_on("task2");
                println!("configuring task 1");
                Ok(())
            })?;

        project
    };

    println!("finished adding tasks to project");

    let freight_args = FreightArgs::command_line("task1 task2 task3 --debug");

    let results = execute_tasks(&project, &freight_args)?;

    println!("{:#?}", results);
    assert_eq!(
        results.iter().map(|result| &result.id).collect::<Vec<_>>(),
        &[
            &project_id.join("task3").unwrap(),
            &project_id.join("task2").unwrap(),
            &project_id.join("task1").unwrap()
        ]
    );
    assert_eq!(results.len(), 3, "all tasks should run");
    assert!(results.iter().all(|r| r.result.is_ok()));
    Ok(())
}
//
// #[test]
// fn detect_task_cycles() -> Result<(), FreightError> {
//     let project = {
//         let mut project = Project::temp(None);
//
//         project
//             .task::<Empty>("task1")?
//             .configure_with(|t, opts, _| {
//                 opts.depend_on("task2");
//                 println!("configuring task 1");
//                 Ok(())
//             });
//         project
//             .task::<Empty>("task2")?
//             .configure_with(|t, opts, _| {
//                 opts.depend_on("task3");
//                 println!("configuring task 2");
//                 Ok(())
//             });
//         project
//             .task::<Empty>("task3")?
//             .configure_with(|t, opts, _| {
//                 println!("configuring task 3");
//                 opts.depend_on("task1");
//                 Ok(())
//             });
//
//         project
//     };
//
//     let freight_args = FreightArgs::command_line("task1 task2 task3 --debug");
//
//     let result = freight_main(&project, freight_args);
//
//     match result {
//         Err(FreightError::ConstructError(ConstructionError::CycleFound { cycle })) => {
//             eprintln!("found cycle consisting of: {:?}", cycle);
//             return Ok(());
//         }
//         Err(other) => {
//             use std::error::Error;
//             println!("error: {}", other);
//             Err(other)
//         }
//         Ok(_) => {
//             panic!("Should be a cyclical construction error")
//         }
//     }
// }
