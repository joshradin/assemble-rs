//! Install component or toolchain with rustup

use std::process::Command;
use log::info;

use assemble_core::exception::BuildException;
use assemble_core::prelude::*;
use assemble_core::lazy_evaluation::{Prop, Provider};
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

        let result = Command::new("rustup")
            .args(&["install", &*toolchain.to_string()]).output()?;
        info!("finished running rustup");
        if !result.status.success()
        {
            warn!("bad result gotten");
            return Err(BuildException::custom("rustup install failed").into());
        }

        Ok(())
    }
}
