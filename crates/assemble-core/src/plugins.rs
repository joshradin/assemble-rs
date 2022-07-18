//! Provide a "unified" way of adding plugins to an assemble project

use crate::dependencies::{Dependency, ToDependency, UnresolvedDependency};
use crate::project::ProjectResult;
use crate::task::Executable;
use crate::{project::Project, BuildResult};
use std::any::type_name;
use std::marker::PhantomData;

/// A plugin to apply to the project. All plugins must implement default.
pub trait Plugin: Default {
    fn apply(&self, project: &mut Project) -> ProjectResult;

    /// The id of the plugin. A plugin of a certain ID can only added once
    fn plugin_id(&self) -> &str {
        type_name::<Self>()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Couldn't create the plugin")]
    CouldNotCreatePlugin,
}
