//! Handles managing and monitoring build initialization
mod descriptor;
mod settings;

use crate::prelude::SharedProject;
use crate::private::Sealed;
use crate::project::ProjectResult;
use crate::Project;
pub use descriptor::*;
use parking_lot::RwLock;
pub use settings::{Settings, SettingsAware};
use std::sync::{Arc, Weak};

/// Trait for creating a project
pub trait CreateProject: Sealed {
    fn create_project(&self) -> ProjectResult<SharedProject>;
}

impl CreateProject for Arc<RwLock<Settings>> {
    fn create_project(&self) -> ProjectResult<SharedProject> {
        let root = self.with_settings(|s| {
            let root = s.root_project();
            create_root_project(self, root)
        })?;
        Ok(root)
    }
}



fn create_project(
    settings: &Arc<RwLock<Settings>>,
    descriptor: &ProjectDescriptor,
    parent: &SharedProject,
) -> ProjectResult<()> {
    let ref root = parent.with(|p| p.root_project());
    parent.with_mut(|parent| parent.subproject_in(
        descriptor.name(),
        descriptor.directory(),
        |p| {
            Ok(())
        }
    ))?;
    let output = parent.with(|parent| parent.get_subproject(descriptor.name()).cloned())?;

    settings.with_settings(|settings_ref| -> ProjectResult<()> {
        for child in settings_ref.children_projects(descriptor) {
            create_project(settings, child, &output)?;
        }
        Ok(())
    })?;
    Ok(())
}

fn create_root_project(
    settings: &Arc<RwLock<Settings>>,
    descriptor: &ProjectDescriptor,
) -> ProjectResult<SharedProject> {
    let mut output = Project::in_dir_with_id_and_root(
        descriptor.directory(),
        descriptor.name(),
        None,
        Some(Arc::downgrade(settings)),
    )?;

    settings.with_settings(|settings_ref|-> ProjectResult<()> {
        for child in settings_ref.children_projects(descriptor) {
            create_project(settings, child, &output)?;
        }
        Ok(())
    })?;
    Ok(output)
}
