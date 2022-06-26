//! Provide a "unified" way of adding plugins to an assemble project

use std::marker::PhantomData;
use crate::dependencies::{Dependency, ToDependency, UnresolvedDependency};
use crate::{BuildResult, Executable, project::Project};



pub trait ToPlugin<T : Executable>
{
    type Plugin: Plugin<T>;

    /// Create a plugin with data from the project.
    fn to_plugin(self, project: &Project<T>) -> Result<Self::Plugin, PluginError>;
}

pub trait Plugin<T: Executable> {
    fn apply(&self, project: &mut Project<T>) -> Result<(), PluginError>;
}

impl <T: Executable, F : Fn(&mut Project<T>) -> Result<(), PluginError>> Plugin<T> for F {
    fn apply(&self, project: &mut Project<T>) -> Result<(), PluginError> {
        (self)(project)
    }
}

impl <T: Executable> ToPlugin<T> for fn(&mut Project<T>) -> Result<(), PluginError> {
    type Plugin = Self;

    fn to_plugin(self, _project: &Project<T>) -> Result<Self::Plugin, PluginError> {
        Ok(self)
    }
}

impl <F : Fn(&mut Project<T>), T: Executable> ToPlugin<T> for F {
    type Plugin = Wrapper<T, F>;

    fn to_plugin(self, _project: &Project<T>) -> Result<Self::Plugin, PluginError> {
        let plugin = Wrapper(self, PhantomData);
        Ok(plugin)
    }
}

pub struct Wrapper<T : Executable, F : Fn(&mut Project<T>)>(F, PhantomData<T>);
impl<T : Executable, F : Fn(&mut Project<T>)> Plugin<T> for Wrapper<T, F> {
    fn apply(&self, project: &mut Project<T>) -> Result<(), PluginError> {
        (self.0)(project);
        Ok(())
    }
}

/// A plugin that's externally configured by a dependency
pub struct ExtPlugin {
    wrapped_dependency: Box<dyn Dependency>,
}

impl<T : Executable> Plugin<T> for ExtPlugin {
    fn apply(&self, project: &mut Project<T>) -> Result<(), PluginError> {
        panic!("External plugins unsupported")
    }
}

impl ExtPlugin {
    pub fn new<T: 'static + Dependency>(wrapped_dependency: T) -> Self {
        Self { wrapped_dependency: Box::new(wrapped_dependency) }
    }
}


#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Couldn't create the plugin")]
    CouldNotCreatePlugin
}

