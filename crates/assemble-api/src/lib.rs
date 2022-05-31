//! The api defines the traits that assemble uses

#[macro_use]
extern crate static_assertions;

#[macro_use]
extern crate serde;

pub mod defaults;
pub mod dependencies;
pub mod exception;
pub mod project;
pub mod resources;
pub mod task;
pub mod utilities;
pub mod web;
pub mod workflow;
pub mod workspace;
