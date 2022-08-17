#![deny(missing_docs)]

//! Provides rust tasks for assemble-projects

#[macro_use]
extern crate assemble_core;

pub mod plugin;
pub mod rustup;
pub mod toolchain;

mod prelude {
    pub use assemble_core::*;
    pub use assemble_std::*;
}
