//! Contains the rust plugin

use crate::extensions::RustPluginExtension;
use crate::rustup::configure_rustup_tasks;
use assemble_core::plugins::extensions::ExtensionAware;
use assemble_core::plugins::Plugin;
use assemble_core::project::error::ProjectResult;
use assemble_core::Project;

/// The rust plugin
#[derive(Debug, Default)]
pub struct RustBasePlugin;

impl RustBasePlugin {
    pub const INSTALL_DEFAULT_TOOLCHAIN: &'static str = "install-default-toolchain";
}

impl Plugin<Project> for RustBasePlugin {
    fn apply_to(&self, project: &mut Project) -> ProjectResult {
        project
            .extensions_mut()
            .add("rust", RustPluginExtension::new())?;
        configure_rustup_tasks(project)
    }
}
