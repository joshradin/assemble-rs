//! Provide a "unified" way of adding plugins to an assemble project

use crate::dependencies::{Dependency, ToDependency, UnresolvedDependency};
use crate::{project::Project, BuildResult, Executable};
use std::marker::PhantomData;

pub trait ToPlugin {
    type Plugin: Plugin;

    /// Create a plugin with data from the project.
    fn to_plugin(self, project: &Project) -> Result<Self::Plugin, PluginError>;
}

pub trait Plugin {
    fn apply(&self, project: &mut Project) -> Result<(), PluginError>;
}

impl<F: Fn(&mut Project) -> Result<(), PluginError>> Plugin for F {
    fn apply(&self, project: &mut Project) -> Result<(), PluginError> {
        (self)(project)
    }
}

impl ToPlugin for fn(&mut Project) -> Result<(), PluginError> {
    type Plugin = Self;

    fn to_plugin(self, _project: &Project) -> Result<Self::Plugin, PluginError> {
        Ok(self)
    }
}

impl<F: Fn(&mut Project)> ToPlugin for F {
    type Plugin = Wrapper<F>;

    fn to_plugin(self, _project: &Project) -> Result<Self::Plugin, PluginError> {
        let plugin = Wrapper(self);
        Ok(plugin)
    }
}

pub struct Wrapper<F: Fn(&mut Project)>(F);
impl<F: Fn(&mut Project)> Plugin for Wrapper<F> {
    fn apply(&self, project: &mut Project) -> Result<(), PluginError> {
        (self.0)(project);
        Ok(())
    }
}

/// A plugin that's externally configured by a dependency
pub struct ExtPlugin {
    wrapped_dependency: Box<dyn Dependency>,
}

impl Plugin for ExtPlugin {
    fn apply(&self, project: &mut Project) -> Result<(), PluginError> {
        panic!("External plugins unsupported")
    }
}

impl ExtPlugin {
    pub fn new<T: 'static + Dependency>(wrapped_dependency: T) -> Self {
        Self {
            wrapped_dependency: Box::new(wrapped_dependency),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Couldn't create the plugin")]
    CouldNotCreatePlugin,
}
