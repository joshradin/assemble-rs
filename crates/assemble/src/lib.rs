#[macro_use]
extern crate log;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate thiserror;

use std::error::Error;
use std::fmt::Display;

use clap::Parser;

use assemble_freight::FreightArgs;

pub mod builders;
pub mod build_logic;


pub fn execute() -> Result<(), Box<dyn Error>> {
    let freight_args = FreightArgs::try_parse_from(wild::args())?;
    println!("args = {:#?}", freight_args);

    Ok(())
}
