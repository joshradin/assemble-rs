#[macro_use]
extern crate serde;

use assemble_api::workflow::BinaryBuilder;
use once_cell::sync::Lazy;

mod binary_building;

mod declarations;
pub mod internal;
mod yaml;

fn get_backend() {
    if cfg!(feature = "rust") {
        todo!()
    } else {
        panic!("no backend defined")
    }
}
