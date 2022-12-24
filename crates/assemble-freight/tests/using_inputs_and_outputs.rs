use assemble_core::defaults::tasks::Empty;
use assemble_core::error::PayloadError;
use assemble_core::identifier::ProjectId;
use log::LevelFilter;

use assemble_core::Project;
use assemble_freight::cli::FreightArgs;
use assemble_freight::utils::FreightError;

use assemble_core::lazy_evaluation::ProviderExt;
use assemble_core::logging::LOGGING_CONTROL;
use assemble_core::prelude::Assemble;
use assemble_freight::ops::{execute_tasks, execute_tasks2};

#[test]
#[ignore]
fn task_ordered_by_dependencies() -> Result<(), PayloadError<FreightError>> {
    assemble_core::logging::init_root_log(LevelFilter::Info, None);

    let project_id = ProjectId::new("test").map_err(PayloadError::new)?;
    let project = {
        let project = Project::with_id(project_id.clone()).map_err(PayloadError::into)?;

        let mut handle = project
            .tasks()
            .register_task::<Empty>("task1")
            .map_err(PayloadError::into)?;
        handle
            .configure_with(|_t, _opts| {
                println!("configuring task 1");
                Ok(())
            })
            .map_err(PayloadError::into)?;
        let _task_id = handle.id();

        project
            .tasks()
            .register_task::<Empty>("task2")
            .map_err(PayloadError::into)?
            .configure_with(move |t, _pro| {
                t.depends_on("task1");
                println!("configuring task 2");
                Ok(())
            })
            .map_err(PayloadError::into)?;

        project
    };

    let freight_args = FreightArgs::command_line("task2");
    let assemble = Assemble::new(freight_args.into());

    let results = execute_tasks2(&project, &assemble)?;

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
