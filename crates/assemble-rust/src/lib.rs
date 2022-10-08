//! Provides rust tasks for assemble-projects

#[macro_use]
extern crate assemble_core;

#[macro_use]
extern crate serde;

#[macro_use]
extern crate log;

use crate::plugin::RustBasePlugin;
use assemble_core::plugins::PluginAware;
use assemble_core::project::ProjectResult;
use assemble_core::Project;

pub mod cargo;
pub mod extensions;
pub mod plugin;
pub mod rustc;
pub mod rustup;
pub mod toolchain;

mod prelude {
    pub use assemble_core::*;
    pub use assemble_std::*;
}

/// The default plugin for rust
#[derive(Debug, Default)]
pub struct Plugin;
impl assemble_core::Plugin<Project> for Plugin {
    fn apply_to(&self, project: &mut Project) -> ProjectResult {
        project.apply_plugin::<RustBasePlugin>()?;
        Ok(())
    }
}
