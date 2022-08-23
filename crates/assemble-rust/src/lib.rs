//! Provides rust tasks for assemble-projects

#[macro_use]
extern crate assemble_core;

#[macro_use]
extern crate serde;

#[macro_use]
extern crate log;

pub mod plugin;
pub mod rustup;
pub mod toolchain;
pub mod extensions;
pub mod cargo;
pub mod rustc;

mod prelude {
    pub use assemble_core::*;
    pub use assemble_std::*;
}
