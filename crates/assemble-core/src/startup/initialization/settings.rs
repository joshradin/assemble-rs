use crate::plugins::PluginAware;
use crate::prelude::{PluginManager, SharedProject};
use crate::startup::initialization::{ProjectBuilder, ProjectDescriptor, ProjectGraph};
use crate::startup::invocation::{Assemble, AssembleAware};
use parking_lot::RwLock;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
#[derive(Debug)]
pub struct Settings {
    assemble: Arc<RwLock<Assemble>>,
    plugin_manager: PluginManager<Settings>,
    project_graph: ProjectGraph,
    root_dir: PathBuf,
    settings_file: PathBuf,
}

impl Settings {
    /// Create a new [`Settings`](Settings) instance.
    pub fn new(
        assemble: &Arc<RwLock<Assemble>>,
        root_dir: PathBuf,
        settings_file: PathBuf,
    ) -> Self {
        Self {
            assemble: assemble.clone(),
            plugin_manager: PluginManager::new(),
            project_graph: ProjectGraph::new(root_dir.clone()),
            root_dir,
            settings_file,
        }
    }

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
        self.add_project(path, |_| {});
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

    /// Gets the child project of a given project
    pub fn children_projects(
        &self,
        proj: &ProjectDescriptor,
    ) -> impl IntoIterator<Item = &ProjectDescriptor> {
        self.project_graph.children_projects(proj)
    }

    /// Gets the root directory of this build
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn set_build_file_name(&mut self, path: impl AsRef<str>) {
        self.project_graph
            .set_default_build_file_name(path.as_ref())
    }

    /// Gets the assemble instance
    pub fn assemble(&self) -> &Arc<RwLock<Assemble>> {
        &self.assemble
    }
    pub fn settings_file(&self) -> &Path {
        &self.settings_file
    }

    /// Gets the project graph
    pub fn project_graph(&self) -> &ProjectGraph {
        &self.project_graph
    }
}

/// A type that's aware of the settings value
pub trait SettingsAware {
    fn with_settings<F: FnOnce(&Settings) -> R, R>(&self, func: F) -> R;
    fn with_settings_mut<F: FnOnce(&mut Settings) -> R, R>(&mut self, func: F) -> R;
}

impl SettingsAware for Settings {
    fn with_settings<F: FnOnce(&Settings) -> R, R>(&self, func: F) -> R {
        (func)(self)
    }

    fn with_settings_mut<F: FnOnce(&mut Settings) -> R, R>(&mut self, func: F) -> R {
        (func)(self)
    }
}

impl SettingsAware for Arc<RwLock<Settings>> {
    fn with_settings<F: FnOnce(&Settings) -> R, R>(&self, func: F) -> R {
        (func)(self.read().deref())
    }

    fn with_settings_mut<F: FnOnce(&mut Settings) -> R, R>(&mut self, func: F) -> R {
        (func)(self.write().deref_mut())
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

impl<S: SettingsAware> AssembleAware for S {
    fn with_assemble<F, R>(&self, func: F) -> R
    where
        F: FnOnce(&Assemble) -> R,
    {
        self.with_settings(|s| s.assemble.with_assemble(func))
    }

    fn with_assemble_mut<F, R>(&mut self, func: F) -> R
    where
        F: FnOnce(&mut Assemble) -> R,
    {
        self.with_settings_mut(|s| s.assemble.with_assemble_mut(func))
    }
}
