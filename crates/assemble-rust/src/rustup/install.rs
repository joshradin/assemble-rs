//! Install component or toolchain with rustup

use log::info;

use assemble_core::exception::BuildException;
use assemble_core::prelude::*;
use assemble_core::properties::{Prop, Provides};
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::task::InitializeTask;
use assemble_core::{BuildResult, Executable, Project, Task};
use assemble_core::{CreateTask, TaskIO};
use assemble_std::ProjectExec;

use crate::toolchain::Toolchain;

/// The toolchain to install
#[derive(Debug, CreateTask, TaskIO)]
pub struct InstallToolchain {
    /// The toolchain to install
    pub toolchain: Prop<Toolchain>,
}

impl UpToDate for InstallToolchain {}

impl InitializeTask for InstallToolchain {}

impl Task for InstallToolchain {
    fn task_action(task: &mut Executable<Self>, project: &Project) -> BuildResult {
        let toolchain = task.toolchain.fallible_get()?;

        info!("attempting to install toolchain {}", toolchain);

        if !project
            .exec_with(|exec| {
                exec.exec("rustup")
                    .arg("install")
                    .arg(toolchain.to_string());
            })?
            .wait()?
            .success()
        {
            return Err(BuildException::custom("rustup install failed").into());
        }

        Ok(())
    }
}
