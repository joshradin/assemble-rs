//! The api defines the traits that assemble uses

#[macro_use]
extern crate static_assertions;

#[macro_use]
extern crate serde;

#[macro_use]
extern crate assemble_macros;

use crate::dependencies::Source;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::marker::PhantomData;

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
