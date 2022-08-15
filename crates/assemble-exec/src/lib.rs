#[macro_use]
extern crate log;
#[macro_use]
extern crate thiserror;
#[macro_use]
extern crate serde;

use assemble_freight::FreightArgs;
use clap::Parser;
use std::error::Error;
use std::fmt::Display;

pub mod builders;

pub fn execute() -> Result<(), Box<dyn Error>> {
    let freight_args = FreightArgs::try_parse_from(wild::args())?;
    println!("args = {:#?}", freight_args);

    Ok(())
}
