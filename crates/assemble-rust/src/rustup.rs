//! Rustup related tasks
//!

use std::fs::File;
use std::path::PathBuf;

use log::{error, info};
use url::Url;

use assemble_core::__export::{CreateTask, InitializeTask, ProjectResult, TaskId, TaskIO};
use assemble_core::defaults::tasks::Basic;
use assemble_core::dependencies::configurations::Configuration;
use assemble_core::exception::{BuildError, BuildException};
use assemble_core::file::RegularFile;
use assemble_core::file_collection::FileCollection;
use assemble_core::plugins::extensions::ExtensionAware;
use assemble_core::prelude::{ProjectError, Provides};
use assemble_core::properties::Prop;
use assemble_core::task::{ExecutableTask, TaskHandle};
use assemble_core::task::up_to_date::UpToDate;
use assemble_std::dependencies::web::{WebDependency, WebRegistry};
use assemble_std::ProjectExec;
use assemble_std::specs::exec_spec::ExecSpec;
use assemble_std::tasks::web::DownloadFile;

use crate::extensions::RustPluginExtension;
use crate::prelude::*;
use crate::rustup::install::InstallToolchain;

pub mod install;
pub mod command;

/// Configure a project to support rustup-related tasks
pub fn configure_rustup_tasks(project: &mut Project) -> Result<(), ProjectError> {
    let mut install = project
        .task_container_mut()
        .register_task::<Empty>("install-rustup")?;

    install.configure_with(|t, _| {
        t.set_description("installs rustup on to the system if not already present");
        t.set_group("rustup");

        t.up_to_date(|_| which::which("rustup").is_ok());

        Ok(())
    })?;

    if cfg!(windows) {
        configure_windows_install(project, install.clone())?;
    } else if cfg!(unix) {
        configure_unix_install(project, install.clone())?;
    }

    project.task_container_mut().register_task_with::<InstallToolchain, _>("install-default-toolchain", |t, p| {
        t.set_description("installs the default toolchain used by this project");
        t.set_group("rustup");

        let extension = p.extension::<RustPluginExtension>().unwrap();
        t.depends_on(install);
        t.toolchain.set_with(extension.toolchain.clone())?;
        Ok(())
    })?;

    Ok(())
}

fn configure_unix_install(project: &mut Project, mut install: TaskHandle<Empty>) -> ProjectResult {
    project.registries_mut(|reg| {
        let registry = WebRegistry::new("rust-site", "https://sh.rustup.rs/").unwrap();
        reg.add_registry(registry);
        Ok(())
    })?;
    let rustup_install_config = project
        .configurations_mut()
        .create_with("rustupInstall", |config| {
            config.add_dependency(
                WebDependency::new("", "rust-site").with_file_name("rustup-init.sh"),
            )
        })
        .clone();

    install.configure_with(move |task, project| {
        task.set_description("installs rustup onto the system");
        task.depends_on(rustup_install_config.clone());
        task.do_first(move |task, project| {
            let configuration = rustup_install_config.resolved()?;
            let rustup_init_file = configuration.files().into_iter().next().unwrap();
            println!("rustup file = {:?}", rustup_init_file);

            match project.exec_with(move |exec| {
                exec.exec("sh")
                    .arg(rustup_init_file)
                    .args(["--default-toolchain", "none"])
                    .args(["--profile", "minimal"])
                    .arg("-y")
                    .arg("-v");
            }) {
                Ok((status, streams)) => {
                    let (out, err) = streams.unwrap();
                    let string = String::from_utf8_lossy(&out);
                    info!("{}", string);
                    let string = String::from_utf8_lossy(&err);
                    error!("{}", string);
                    if !status.success() {
                        return Err(BuildException::custom(
                            "installing rustup fail. Check console log for more info.",
                        ));
                    }
                }
                Err(e) => return Err(BuildException::from(e)),
            }

            Ok(())
        })?;
        Ok(())
    })?;

    Ok(())
}

fn configure_windows_install(
    project: &mut Project,
    mut install: TaskHandle<Empty>,
) -> ProjectResult {
    project.registries_mut(|reg| {
        let registry = WebRegistry::new("rust-site", "https://static.rust-lang.org/").unwrap();
        reg.add_registry(registry);
        Ok(())
    })?;
    let rustup_install_config = project
        .configurations_mut()
        .create_with("rustup-install", |config| {
            #[cfg(target_pointer_width = "64")]
            config.add_dependency(WebDependency::new(
                "/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe",
                "rust-site",
            ));
            #[cfg(target_pointer_width = "32")]
            config.add_dependency(WebDependency::new(
                "/rustup/dist/i686-pc-windows-msvc/rustup-init.exe",
                "rust-site",
            ));
        })
        .clone();

    install.configure_with(move |task, project| {
        task.set_description("installs rustup onto the system");
        task.depends_on(rustup_install_config.clone());
        task.do_first(move |task, project| {
            let configuration = rustup_install_config.resolved()?;
            let rustup_init_file = configuration.files().into_iter().next().unwrap();
            println!("rustup file = {:?}", rustup_init_file);

            match project.exec_with(move |exec| {
                exec.exec(rustup_init_file)
                    .args(["--default-toolchain", "none"])
                    .args(["--profile", "minimal"])
                    .arg("-y")
                    .arg("-v");
            }) {
                Ok((status, streams)) => {
                    let (out, err) = streams.unwrap();
                    let string = String::from_utf8_lossy(&out);
                    info!("{}", string);
                    let string = String::from_utf8_lossy(&err);
                    error!("{}", string);
                    if !status.success() {
                        return Err(BuildException::custom(
                            "installing rustup fail. Check console log for more info.",
                        ));
                    }
                }
                Err(e) => return Err(BuildException::from(e)),
            }

            Ok(())
        })?;
        Ok(())
    })?;

    Ok(())
}
