#[macro_use]
extern crate serde;

use once_cell::sync::Lazy;

pub mod binary_building;

pub mod declarations;
pub mod internal;
pub mod yaml;

fn get_backend() {
    if cfg!(feature = "rust") {
        todo!()
    } else {
        panic!("no backend defined")
    }
}
