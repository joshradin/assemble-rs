//! Build a rust project

use crate::cargo::Target;
use crate::extensions::RustPluginExtension;
use crate::prelude::*;
use crate::toolchain::Toolchain;
use assemble_core::lazy_evaluation::{Prop, VecProp};
use assemble_core::plugins::extensions::ExtensionAware;
use assemble_core::project::error::ProjectResult;
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::task::InitializeTask;

/// A task to build rust projects
#[derive(Debug, CreateTask, TaskIO)]
pub struct CargoFmt {
    /// The toolchain of the cargo build
    pub toolchain: Prop<Toolchain>,
    /// The targets to use while building
    pub targets: VecProp<Target>,
}

impl InitializeTask for CargoFmt {
    fn initialize(task: &mut Executable<Self>, project: &Project) -> ProjectResult {
        let ext = project.extension::<RustPluginExtension>().unwrap();
        task.toolchain.set_with(ext.toolchain.clone())?;
        Ok(())
    }
}

impl UpToDate for CargoFmt {}

impl Task for CargoFmt {
    fn task_action(_task: &mut Executable<Self>, _project: &Project) -> BuildResult {
        todo!()
    }
}
