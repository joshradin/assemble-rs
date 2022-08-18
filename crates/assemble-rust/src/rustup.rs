//! Rustup related tasks
//!

use crate::prelude::*;
use assemble_core::__export::{CreateTask, InitializeTask, ProjectResult, TaskIO, TaskId};
use assemble_core::exception::{BuildError, BuildException};
use assemble_core::file::RegularFile;
use assemble_core::prelude::Provides;
use assemble_core::properties::Prop;
use assemble_core::task::up_to_date::UpToDate;
use assemble_std::ProjectExec;
use std::path::PathBuf;
use url::Url;

/// Installs RustUp itself
#[derive(Debug, CreateTask, TaskIO)]
pub struct InstallRustUpInit {
    /// The url where the script for installing rustup is located
    #[input]
    pub script_url: Prop<Url>,
    /// The directory to download the file into
    #[output]
    pub install_script_location: Prop<PathBuf>,
}

impl UpToDate for InstallRustUpInit {
    /// Detects if rust-up is installed on the system
    fn up_to_date(&self) -> bool {
        todo!()
    }
}

impl InitializeTask for InstallRustUpInit {
    fn initialize(task: &mut Executable<Self>, project: &Project) -> ProjectResult {
        task.script_url
            .set(Url::parse("https://sh.rustup.rs").unwrap())?;
        task.install_script_location
            .set(project.root_dir().join(".rust"))?;
        Ok(())
    }
}

impl Task for InstallRustUpInit {
    fn task_action(task: &mut Executable<Self>, project: &Project) -> BuildResult {
        let file = task.install_script_location.get();
        let result = project.exec(|config| {
            config
                .exec("curl")
                .args(["--proto", "=https"])
                .arg("-sSf")
                .args(["--output", &format!("{:?}", file)])
                .arg("https://sh.rustup.rs");
        })?;

        if !result.success() {
            return Err(BuildException::new("could not download rustup"));
        }

        Ok(())
    }
}
