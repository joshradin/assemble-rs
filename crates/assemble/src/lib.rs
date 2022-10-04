#[macro_use]
extern crate assemble_core;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate thiserror;

use std::env::current_dir;
use std::error::Error;
use std::fmt::Display;
use std::panic;
use std::process::exit;

use anyhow::anyhow;
use anyhow::Result;

use crate::build_logic::plugin::{BuildLogicExtension, BuildLogicPlugin};
use assemble_core::lazy_evaluation::Provider;
use assemble_core::logging::LOGGING_CONTROL;
use assemble_core::plugins::extensions::ExtensionAware;
use assemble_core::prelude::{ProjectResult, SharedProject, TaskId};
use assemble_core::text_factory::list::{Counter, MultiLevelBulletFactory, TextListFactory};
use assemble_core::text_factory::AssembleFormatter;
use assemble_core::utilities::measure_time;
use assemble_core::{execute_assemble, Project};
use assemble_freight::ops::execute_tasks;
use assemble_freight::utils::TaskResult;
use assemble_freight::FreightArgs;

use crate::builders::BuildSettings;

pub mod build_logic;
pub mod builders;
#[cfg(debug_assertions)]
pub mod dev;

pub fn execute() -> std::result::Result<(), ()> {
    let freight_args: FreightArgs = FreightArgs::from_env();
    let join_handle = freight_args
        .logging()
        .init_root_logger()
        .map_err(|_| ())?
        .expect("this should be top level entry");

    let output = with_args(freight_args);

    let output = if let Err(e) = output {
        error!("{}", e);
        Err(())
    } else {
        Ok(())
    };
    LOGGING_CONTROL.stop_logging();
    join_handle.join().expect("should be able to join here");
    output
}

pub fn with_args(freight_args: FreightArgs) -> Result<()> {
    let join_handle = freight_args.logging().init_root_logger();
    let properties = freight_args.properties().properties();

    let ret = measure_time(
        "total execution",
        log::Level::Debug,
        || -> Result<()> {
            let build_logic: SharedProject = if cfg!(feature = "yaml") {
                #[cfg(feature = "yaml")]
                {
                    use builders::yaml::yaml_build_logic::YamlBuilder;
                    let build_logic = YamlBuilder.discover(current_dir()?, &properties)?;
                    build_logic
                }
                #[cfg(not(feature = "yaml"))]
                unreachable!()
            } else {
                panic!("No builder defined")
            };

            let ref build_logic_args =
                freight_args.with_tasks([BuildLogicPlugin::COMPILE_SCRIPTS_TASK]);
            let results = execute_tasks(&build_logic, build_logic_args)?;
            let mut failed_tasks = vec![];

            emit_task_results(results, &mut failed_tasks, freight_args.backtrace());

            if failed_tasks.is_empty() {
                debug!("dynamically loading the compiled build logic project");
                let path = build_logic.with(|t| {
                    let ext = t.extension::<BuildLogicExtension>().unwrap();
                    ext.built_library.fallible_get()
                })?;
                debug!("library path: {:?}", path);
                let project = unsafe {
                    let lib =
                        libloading::Library::new(path).expect("couldn't load dynamic library");
                    debug!("loaded lib: {:?}", lib);
                    let build_project = lib
                        .get::<fn(&SharedProject) -> ProjectResult>(b"configure_project")
                        .expect("no configure_project symbol");

                    let project = Project::new()?;
                    build_project(&project)?;
                    project
                };
                let results = execute_tasks(&project, &freight_args)?;
                emit_task_results(results, &mut failed_tasks, freight_args.backtrace());
            }

            if !failed_tasks.is_empty() {
                return Err(anyhow!("tasks failed: {:?}", failed_tasks));
            }
            Ok(())
        },
    );

    if let Ok(Some(join_h)) = join_handle {
        LOGGING_CONTROL.stop_logging();
        join_h.join().expect("should be able to join here")
    }

    ret
}

/// Emits task results.
///
/// extends a list of failed task ids
fn emit_task_results(results: Vec<TaskResult>, failed: &mut Vec<TaskId>, show_backtrace: bool) {
    let mut list = TextListFactory::new("> ");

    for task_r in results {
        if task_r.result.is_err() {
            let result = task_r.result;
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
            failed.push(task_r.id);
        }
    }

    let list = list.finish();
    if !list.is_empty() {
        error!("");
        error!("{}", list);
        error!("");
    }
}
