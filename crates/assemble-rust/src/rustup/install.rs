//! Install component or toolchain with rustup

use log::Level;

use assemble_core::exception::BuildException;
use assemble_core::lazy_evaluation::{Prop, Provider};

use assemble_core::task::initialize_task::InitializeTask;
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::{BuildResult, Executable, Project, Task};
use assemble_core::{CreateTask, TaskIO};
use assemble_std::ProjectExec;

use crate::toolchain::Toolchain;

/// The toolchain to install
#[derive(Debug, CreateTask, TaskIO)]
pub struct InstallToolchain {
    /// The toolchain to install
    #[input]
    pub toolchain: Prop<Toolchain>,
}

impl UpToDate for InstallToolchain {}

impl InitializeTask for InstallToolchain {}

impl Task for InstallToolchain {
    fn task_action(task: &mut Executable<Self>, project: &Project) -> BuildResult {
        let toolchain = task.toolchain.fallible_get()?;

        debug!("attempting to install toolchain {}", toolchain);

        if !project
            .exec_with(|exec| {
                exec.exec("rustup")
                    .arg("install")
                    .arg("--no-self-update")
                    .arg(toolchain.to_string())
                    .stdout(Level::Debug);
            })?
            .success()
        {
            warn!("bad result gotten");
            return Err(BuildException::custom("rustup install failed").into());
        }

        Ok(())
    }
}
