#[macro_use]
extern crate assemble_core;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;

use std::panic;
use std::sync::Arc;

use parking_lot::RwLock;
use assemble_core::error::PayloadError;

use assemble_core::logging::LOGGING_CONTROL;
use assemble_core::prelude::{
    self, Assemble, CreateProject, Settings, SharedProject, StartParameter, TaskId,
};
use assemble_core::text_factory::list::TextListFactory;
use assemble_core::Project;
use assemble_freight::utils::{FreightError, TaskResult};
use assemble_freight::{init_assemble, FreightArgs};
use build_logic::BuildLogic;

use crate::builders::BuildConfigurator;

pub mod build_logic;
pub mod builders;
#[cfg(debug_assertions)]
pub mod dev;
pub mod error;

pub fn execute_v2() -> std::result::Result<(), ()> {
    let freight_args: FreightArgs = FreightArgs::from_env();
    let join_handle = freight_args
        .logging()
        .init_root_logger()
        .map_err(|_| ())?
        .expect("this should be top level entry");

    let mut start_param = StartParameter::from(freight_args);

    trace!("start param: {:#?}", start_param);
    let builder = builders::builder();
    let show_backtrace = start_param.backtrace();

    let output = build(start_param, &builder);

    let output = if let Err(e) = output {
        if e.is::<PayloadError<FreightError>>() {
            let downcast = e.downcast::<PayloadError<FreightError>>().unwrap();
            error!("{:#}", downcast);
            if show_backtrace {
                error!("{:?}", downcast.backtrace());
            }
            Err(())
        } else {
            error!("{:}", e);
            Err(())
        }

    } else {
        Ok(())
    };
    LOGGING_CONTROL.stop_logging();
    join_handle.join().expect("should be able to join here");
    output
}

pub fn build<B: BuildConfigurator>(
    start_parameter: StartParameter,
    builder: &B,
) -> anyhow::Result<()>
where
    B::Err: 'static,
{
    let join_handle = start_parameter.logging().init_root_logger();
    let _properties = start_parameter.properties();

    let assemble: Arc<RwLock<Assemble>> = Arc::new(RwLock::new(
        init_assemble(start_parameter).expect("couldn't init assemble"),
    ));
    trace!("assemble: {:#?}", assemble);

    let ret = (move || -> anyhow::Result<()> {
        let mut settings: Arc<RwLock<Settings>> = Arc::new(RwLock::new(
            builder.discover(assemble.read().current_dir(), &assemble)?,
        ));

        builder.configure_settings(&mut settings)?;
        trace!("settings: {:#?}", settings);
        trace!("project graph:\n{}", settings.read().project_graph());

        let mut build_logic = configure_build_logic(&settings, builder)?;
        let project = CreateProject::create_project(&settings)?;

        build_logic.configure(&settings, &project)?;

        Ok(())
    })();

    if let Ok(Some(join_h)) = join_handle {
        LOGGING_CONTROL.stop_logging();
        join_h.join().expect("should be able to join here")
    }

    ret
}

fn configure_build_logic<B: BuildConfigurator>(
    settings: &Arc<RwLock<Settings>>,
    builder: &B,
) -> Result<B::BuildLogic<Arc<RwLock<Settings>>>, B::Err> {
    builder.get_build_logic(settings)
}

//
// pub fn with_args(freight_args: FreightArgs) -> Result<()> {
//     let join_handle = freight_args.logging().init_root_logger();
//     let properties = freight_args.properties().properties();
//
//     let mut assemble = Arc::new(RwLock::new(init_assemble(freight_args).expect("couldn't init assemble")));
//
//     let ret = (|| -> Result<()> {
//         let start = Instant::now();
//
//         let build_logic: SharedProject = if cfg!(feature = "yaml") {
//             #[cfg(feature = "yaml")]
//             {
//                 use builders::yaml::yaml_build_logic::YamlBuilder;
//
//                 YamlBuilder.discover(current_dir()?, &assemble)?
//             }
//             #[cfg(not(feature = "yaml"))]
//             unreachable!()
//         } else {
//             panic!("No builder defined")
//         };
//
//         let build_logic_args = &freight_args.with_tasks([BuildLogicPlugin::COMPILE_SCRIPTS_TASK]);
//         let mut results = execute_tasks(&build_logic, build_logic_args)?;
//         let mut failed_tasks = vec![];
//
//         emit_task_results(&results, &mut failed_tasks, freight_args.backtrace());
//
//         if failed_tasks.is_empty() {
//             debug!("dynamically loading the compiled build logic project");
//             let path = build_logic.with(|t| {
//                 let ext = t.extension::<BuildLogicExtension>().unwrap();
//                 ext.built_library.fallible_get()
//             })?;
//             debug!("library path: {:?}", path);
//             let project = unsafe {
//                 let lib = libloading::Library::new(path).expect("couldn't load dynamic library");
//                 debug!("loaded lib: {:?}", lib);
//                 let build_project = lib
//                     .get::<fn(&SharedProject) -> ProjectResult>(b"configure_project")
//                     .expect("no configure_project symbol");
//
//                 let project = Project::new()?;
//                 build_project(&project)?;
//                 project
//             };
//             let actual_results = execute_tasks(&project, &freight_args)?;
//             emit_task_results(&actual_results, &mut failed_tasks, freight_args.backtrace());
//             results.extend(actual_results);
//         }
//
//         let status = BuildResultString::new(failed_tasks.is_empty(), start.elapsed());
//
//         info!("{}", status);
//
//         let (executed, up_to_date) =
//             results
//                 .iter()
//                 .map(|r| &r.outcome)
//                 .fold((0, 0), |(executed, up_to_date), outcome| match outcome {
//                     TaskOutcome::Executed => (executed + 1, up_to_date),
//                     TaskOutcome::Skipped | TaskOutcome::UpToDate | TaskOutcome::NoSource => {
//                         (executed, up_to_date + 1)
//                     }
//                     TaskOutcome::Failed => (executed, up_to_date),
//                 });
//         info!(
//             "{} actionable tasks: {} executed, {} up-to-date",
//             executed + up_to_date,
//             executed,
//             up_to_date
//         );
//
//         if !failed_tasks.is_empty() {
//             return Err(anyhow!("tasks failed: {:?}", failed_tasks));
//         }
//         Ok(())
//     })();
//
//     if let Ok(Some(join_h)) = join_handle {
//         LOGGING_CONTROL.stop_logging();
//         join_h.join().expect("should be able to join here")
//     }
//
//     ret
// }

/// Emits task results.
///
/// extends a list of failed task ids
fn emit_task_results(results: &Vec<TaskResult>, failed: &mut Vec<TaskId>, show_backtrace: bool) {
    let mut list = TextListFactory::new("> ");

    for task_r in results {
        if task_r.result.is_err() {
            let result = task_r.result.as_ref();
            let err = result.unwrap_err();
            list = list
                .element(format!("Task {} failed", task_r.id))
                .sublist(|sub| {
                    let sub = sub.element(format!("{}", err.kind()));
                    if show_backtrace {
                        sub.element(format!("{:?}", err.backtrace()))
                    } else {
                        sub
                    }
                });
            failed.push(task_r.id.clone());
        }
    }

    let list = list.finish();
    if !list.is_empty() {
        error!("");
        error!("{}", list);
        error!("");
    }
}
