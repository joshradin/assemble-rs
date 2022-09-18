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

use anyhow::anyhow;
use anyhow::Result;

use assemble_core::execute_assemble;
use assemble_core::logging::LOGGING_CONTROL;
use assemble_core::prelude::{SharedProject, TaskId};
use assemble_core::text_factory::list::{Counter, MultiLevelBulletFactory, TextListFactory};
use assemble_core::text_factory::AssembleFormatter;
use assemble_core::utilities::measure_time;
use assemble_freight::ops::execute_tasks;
use assemble_freight::utils::TaskResult;
use assemble_freight::FreightArgs;
use crate::build_logic::plugin::BuildLogicPlugin;

use crate::build_logic::plugin::BuildLogicPlugin;
use crate::builders::BuildSettings;

pub mod build_logic;
pub mod builders;
#[cfg(debug_assertions)]
pub mod dev;

pub fn execute() -> std::result::Result<(), ()> {
    let freight_args: FreightArgs = FreightArgs::from_env();
    let join_handle = freight_args
        .logging
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
    let join_handle = freight_args.logging.init_root_logger();
    let properties = freight_args.properties.properties();

    measure_time(
        ":build-logic project execution",
        log::Level::Info,
        || ->  Result<()> {
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
                FreightArgs::command_line(
                    format!("{} --workers {}", BuildLogicPlugin::COMPILE_SCRIPTS_TASK, freight_args.workers));
            let results = execute_tasks(&build_logic, build_logic_args)?;
            let mut failed_tasks = vec![];

            emit_task_results(results, &mut failed_tasks, freight_args.backtrace);
            if !failed_tasks.is_empty() {
                return Err(anyhow!("tasks failed: {:?}", failed_tasks));
            }
            Ok(())
        },
    )?;

    if let Ok(Some(join_h)) = join_handle {
        LOGGING_CONTROL.stop_logging();
        join_h.join().expect("should be able to join here")
    }

    Ok(())
}

/// Emits task results.
///
/// extends a list of failed task ids
fn emit_task_results(results: Vec<TaskResult>, failed: &mut Vec<TaskId>, show_backtrace: bool) {
    let mut list =
        TextListFactory::new("> ");

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
