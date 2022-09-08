#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate thiserror;
#[macro_use]
extern crate assemble_core;

use std::env::current_dir;
use std::error::Error;
use std::fmt::Display;
use std::panic;

use assemble_core::execute_assemble;
use assemble_core::logging::LOGGING_CONTROL;
use assemble_core::prelude::SharedProject;
use assemble_core::utilities::measure_time;

use crate::builders::BuildSettings;
use assemble_freight::ops::execute_tasks;
use assemble_freight::FreightArgs;

pub mod build_logic;
pub mod builders;

pub fn execute() -> Result<(), ()> {
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

pub fn with_args(freight_args: FreightArgs) -> Result<(), Box<dyn Error>> {
    let join_handle = freight_args.logging.init_root_logger();
    let properties = freight_args.properties.properties();

    measure_time(
        ":build-logic project execution",
        log::Level::Info,
        || -> Result<(), Box<dyn Error>> {
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

            let ref build_logic_args = FreightArgs::command_line("createCargoToml");
            execute_tasks(&build_logic, build_logic_args)?;
            Ok(())
        },
    )?;



    if let Ok(Some(join_h)) = join_handle {
        LOGGING_CONTROL.stop_logging();
        join_h.join().expect("should be able to join here")
    }

    Ok(())
}
