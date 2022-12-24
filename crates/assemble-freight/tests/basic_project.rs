use assemble_core::defaults::tasks::Empty;
use assemble_core::error::PayloadError;
use assemble_core::identifier::ProjectId;
use assemble_core::Project;
use assemble_freight::cli::FreightArgs;

use assemble_freight::ops::execute_tasks;
use assemble_freight::utils::FreightError;

#[test]
#[ignore] // uses old mechanism
fn resolve_and_execute_project() -> Result<(), PayloadError<FreightError>> {
    let project_id = ProjectId::new("test").map_err(PayloadError::new)?;
    let project = {
        let project = Project::with_id(project_id.clone()).map_err(PayloadError::into)?;

        project
            .tasks()
            .register_task::<Empty>("task3")
            .map_err(PayloadError::into)?
            .configure_with(|_t, _opts| {
                println!("configuring task 3");
                Ok(())
            })
            .map_err(PayloadError::into)?;
        project
            .tasks()
            .register_task::<Empty>("task2")
            .map_err(PayloadError::into)?
            .configure_with(|t, _opts| {
                t.depends_on("task3");
                println!("configuring task 2");
                Ok(())
            })
            .map_err(PayloadError::into)?;
        project
            .tasks()
            .register_task::<Empty>("task1")
            .map_err(PayloadError::into)?
            .configure_with(|t, _opts| {
                t.depends_on("task2");
                println!("configuring task 1");
                Ok(())
            })
            .map_err(PayloadError::into)?;

        project
    };

    println!("finished adding tasks to project");

    let freight_args = FreightArgs::command_line("task1 task2 task3 --debug");
    eprintln!("args: {:#?}", freight_args);

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
