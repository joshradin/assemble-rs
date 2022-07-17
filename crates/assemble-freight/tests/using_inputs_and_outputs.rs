use assemble_core::identifier::ProjectId;
use assemble_core::task::{BuildableTask, Empty};
use assemble_core::Project;
use assemble_freight::utils::FreightError;
use assemble_freight::{freight_main, FreightArgs};

use assemble_core::properties::ProvidesExt;

#[test]
fn task_ordered_by_dependencies() -> Result<(), FreightError> {
    let project_id = ProjectId::new("test")?;
    let project = {
        let mut project = Project::with_id(project_id.clone())?;

        let mut handle = project.tasks().register_task::<Empty>("task1")?;
        handle.configure_with(|t, opts| {
            println!("configuring task 1");
            Ok(())
        })?;
        let task_id = handle.id();

        project
            .tasks()
            .register_task::<Empty>("task2")?
            .configure_with(move |t, pro| {
                println!("configuring task 2");
                Ok(())
            })?;

        project
    };

    let freight_args = FreightArgs::command_line("task2");

    let results = freight_main(&project, freight_args)?;

    println!("{:#?}", results);
    assert_eq!(
        results.iter().map(|result| &result.id).collect::<Vec<_>>(),
        &[
            &project_id.join("task1").unwrap(),
            &project_id.join("task2").unwrap(),
        ]
    );
    assert!(results.iter().all(|r| r.result.is_ok()));
    Ok(())
}
