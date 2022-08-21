//! Build a rust project

use assemble_core::properties::Prop;
use crate::prelude::*;
use crate::toolchain::Toolchain;

/// A task to build rust projects
#[derive(Debug, CreateTask, TaskIO)]
pub struct CargoBuild {
    pub toolchain: Prop<Toolchain>,
    pub targets: Prop<Toolchain>
}