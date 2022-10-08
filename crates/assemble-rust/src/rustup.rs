//! Rustup related tasks
//!

use log::info;

use assemble_core::dependencies::configurations::Configuration;
use assemble_core::exception::BuildException;

use assemble_core::file_collection::FileCollection;

use assemble_core::plugins::extensions::ExtensionAware;

use assemble_core::project::error::{ProjectError, ProjectResult};
use assemble_core::task::create_task::CreateTask;

use assemble_core::task::up_to_date::UpToDate;

use assemble_std::dependencies::web::{WebDependency, WebRegistry};
use assemble_std::specs::exec_spec::Output;

use assemble_std::ProjectExec;

use crate::extensions::RustPluginExtension;
use crate::plugin::RustBasePlugin;
use crate::prelude::*;
use crate::rustup::install::InstallToolchain;

pub mod install;

/// Configure a project to support rustup-related tasks
pub fn configure_rustup_tasks(project: &mut Project) -> ProjectResult<()> {
    let mut install = project
        .task_container_mut()
        .register_task::<Empty>("install-rustup")?;

    install.configure_with(|t, _| {
        t.set_description("installs rustup on to the system if not already present");
        t.set_group("rustup");

        t.up_to_date(|_| which::which("rustup").is_ok());

        Ok(())
    })?;

    let rustup_install_config = if cfg!(windows) {
        configure_windows_install(project)?
    } else if cfg!(unix) {
        configure_unix_install(project)?
    } else {
        return Err(ProjectError::custom("unsupported os for rustup").into());
    };

    install.configure_with(move |task, _project| {
        task.depends_on(rustup_install_config.clone());
        task.do_first(move |_task, project| {
            if which::which("rustup").is_ok() {
                return Err(BuildException::StopTask.into());
            }

            let configuration = rustup_install_config.resolved()?;
            let rustup_init_file = configuration.files().into_iter().next().unwrap();
            println!("rustup file = {:?}", rustup_init_file);

            match project.exec_with(move |exec| {
                exec.exec(rustup_init_file)
                    .args(["--default-toolchain", "none"])
                    .args(["--profile", "minimal"])
                    .arg("-v")
                    .stdout(Output::Bytes)
                    .stderr(Output::Bytes);
            }) {
                Ok(handle) => {
                    let string = handle.utf8_string_err().unwrap()?;
                    info!("rustup log: {}", string);
                    if string.contains("error: cannot install while Rust is installed") {
                        info!("assuming ok");
                        return Ok(());
                    }
                    if !handle.success() {
                        return Err(BuildException::custom(
                            "installing rustup fail. Check console log for more info.",
                        )
                        .into());
                    }
                }
                Err(e) => return Err(BuildException::from(e).into()),
            }

            Ok(())
        })?;

        Ok(())
    })?;
    project
        .task_container_mut()
        .register_task_with::<InstallToolchain, _>(
            RustBasePlugin::INSTALL_DEFAULT_TOOLCHAIN,
            |t, p| {
                t.set_description("installs the default toolchain used by this project");
                t.set_group("rustup");

                let extension = p.extension::<RustPluginExtension>().unwrap();
                t.depends_on(install);
                t.toolchain.set_with(extension.toolchain.clone())?;
                Ok(())
            },
        )?;

    Ok(())
}

fn configure_unix_install(project: &mut Project) -> ProjectResult<Configuration> {
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

    Ok(rustup_install_config)
}

fn configure_windows_install(project: &mut Project) -> ProjectResult<Configuration> {
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

    Ok(rustup_install_config)
}
