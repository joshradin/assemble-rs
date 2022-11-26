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
        let weak = Arc::downgrade(self);

        let root = self.with_settings(|s| create_root_project(weak, s.root_project()))?;
        todo!()
    }
}

fn create_project(
    settings: Weak<RwLock<Settings>>,
    descriptor: &ProjectDescriptor,
    parent: &SharedProject,
) -> ProjectResult<SharedProject> {
    let ref root = parent.with(|p| p.root_project());
    let mut output = Project::in_dir_with_id_and_root(
        descriptor.directory(),
        descriptor.name(),
        Some(root),
        Some(settings),
    )?;
    output.with_mut(|p| p.set_parent(parent));
    Ok(output)
}

fn create_root_project(
    settings: Weak<RwLock<Settings>>,
    descriptor: &ProjectDescriptor,
) -> ProjectResult<SharedProject> {
    Project::in_dir_with_id_and_root(
        descriptor.directory(),
        descriptor.name(),
        None,
        Some(settings),
    )
}
