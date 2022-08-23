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

use clap::Parser;
use assemble_core::execute_assemble;
use assemble_core::logging::LOGGING_CONTROL;

use assemble_freight::FreightArgs;
use assemble_freight::ops::execute_tasks;
use crate::builders::BuildSettings;


pub mod build_logic;
pub mod builders;

pub fn execute() -> Result<(), ()> {
    let freight_args: FreightArgs = FreightArgs::try_parse_from(wild::args()).map_err(|_| ())?;
    let join_handle = freight_args.logging.init_root_logger().map_err(|_| ())?.expect("this should be top level entry");

    println!("args = {:#?}", freight_args);
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

    let build_logic = if cfg!(feature="yaml") {
        #[cfg(feature = "yaml")]
        {
            use builders::yaml::yaml_build_logic::YamlBuilder;
            let build_logic = YamlBuilder.discover(
                current_dir()?,
                &properties
            )?;
            build_logic
        }
    } else {
        panic!("No builder defined")
    };

    let ref build_logic_args = FreightArgs::command_line("compileScripts");
    execute_tasks(&build_logic, build_logic_args)?;

    if let Ok(Some(join_h)) = join_handle {
        LOGGING_CONTROL.stop_logging();
        join_h.join().expect("should be able to join here")
    }

    Ok(())
}
