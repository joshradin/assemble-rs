//! Build a rust project

use assemble_core::__export::ProjectResult;
use assemble_core::plugins::extensions::ExtensionAware;
use assemble_core::properties::{Prop, VecProp};
use assemble_core::task::InitializeTask;
use assemble_core::task::up_to_date::UpToDate;
use crate::cargo::Target;
use crate::extensions::RustPluginExtension;
use crate::prelude::*;
use crate::toolchain::Toolchain;

/// A task to build rust projects
#[derive(Debug, CreateTask, TaskIO)]
pub struct CargoFmt {
    /// The toolchain of the cargo build
    pub toolchain: Prop<Toolchain>,
    /// The targets to use while building
    pub targets: VecProp<Target>
}

impl InitializeTask for CargoFmt {
    fn initialize(task: &mut Executable<Self>, project: &Project) -> ProjectResult {
        let ext = project.extension::<RustPluginExtension>().unwrap();
        task.toolchain.set_with(ext.toolchain.clone())?;
        Ok(())
    }
}

impl UpToDate for CargoFmt {

}

impl Task for CargoFmt {
    fn task_action(_task: &mut Executable<Self>, _project: &Project) -> BuildResult {
        todo!()
    }
}