//! Contains the rust plugin

use crate::rustup::configure_rustup_tasks;
use assemble_core::__export::ProjectResult;
use assemble_core::plugins::extensions::ExtensionAware;
use assemble_core::plugins::Plugin;
use assemble_core::Project;
use crate::extensions::RustPluginExtension;

/// The rust plugin
#[derive(Debug, Default)]
pub struct RustPlugin;

impl Plugin for RustPlugin {
    fn apply(&self, project: &mut Project) -> ProjectResult {
        project.extensions_mut().add("rust", RustPluginExtension::new())?;
        configure_rustup_tasks(project)
    }
}
