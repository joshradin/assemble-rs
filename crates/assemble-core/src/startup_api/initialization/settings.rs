use crate::plugins::PluginAware;
use crate::prelude::PluginManager;
use crate::startup_api::initialization::{ProjectBuilder, ProjectDescriptor, ProjectGraph};
use crate::startup_api::invocation::{Assemble, AssembleAware};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use toml_edit::Item;

/// Declares the configuration required to instantiate and configure the hierarchy of [`SharedProject`](crate::project::SharedProject)
/// which are part of this build. There's exactly one settings instance that's created per
/// settings file.
///
/// # Assembling a mutli-project build
/// One of the purposes of the `Settings` object is to allow you to declare projects which are
/// included in this build.
///
/// When included, a [`ProjectDescriptor`][pd] is created which can be used to configure the default
/// values for several properties of the project.
///
/// [pd]: super::descriptor::ProjectDescriptor
///
/// # Using Settings in a Settings File
/// Depends on the builder..
///
pub struct Settings {
    assemble: Arc<Assemble>,
    plugin_manager: PluginManager<Settings>,
    project_graph: ProjectGraph,
    root_dir: PathBuf,
}

impl Settings {
    /// Gets the root project descriptor
    pub fn root_project(&self) -> &ProjectDescriptor {
        self.project_graph.root_project()
    }

    /// Gets a mutable reference to the root project descriptor
    pub fn root_project_mut(&mut self) -> &mut ProjectDescriptor {
        self.project_graph.root_project_mut()
    }

    /// Adds a child project to the root project
    pub fn add_project<S: AsRef<str>, F: FnOnce(&mut ProjectBuilder)>(
        &mut self,
        path: S,
        configure: F,
    ) {
        self.project_graph.project(path, configure)
    }

    /// Includes a project a path.
    pub fn include<S: AsRef<str>>(&mut self, path: S) {
        self.add_project(path, |_| {})
    }

    /// Includes a project a path.
    pub fn include_all<S: AsRef<str>, I: IntoIterator<Item = S>>(&mut self, paths: I) {
        for path in paths {
            self.include(path)
        }
    }

    /// Find a project within this build
    pub fn find_project(&self, path: impl AsRef<Path>) -> Option<&ProjectDescriptor> {
        self.project_graph.find_project(path)
    }

    /// Find a project within this build
    pub fn find_project_mut(&mut self, path: impl AsRef<Path>) -> Option<&mut ProjectDescriptor> {
        self.project_graph.find_project_mut(path)
    }

    /// Gets the root directory of this build
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }
}

impl AssembleAware for Settings {
    fn get_assemble(&self) -> &Assemble {
        self.assemble.as_ref()
    }
}

/// A type that's aware of the settings value
pub trait SettingsAware {
    /// Gets the settings value that this value is aware of
    fn get_settings(&self) -> &Settings;
}

impl SettingsAware for Settings {
    /// Gets this instance of settings
    fn get_settings(&self) -> &Settings {
        &self
    }
}

impl PluginAware for Settings {
    fn plugin_manager(&self) -> &PluginManager<Self> {
        &self.plugin_manager
    }

    fn plugin_manager_mut(&mut self) -> &mut PluginManager<Self> {
        &mut self.plugin_manager
    }
}
