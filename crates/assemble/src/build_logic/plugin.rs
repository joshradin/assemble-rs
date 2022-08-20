//! The build-logic plugin that the :build-logic project adds

use assemble_core::plugins::Plugin;
use assemble_core::prelude::*;

pub mod script;

#[derive(Debug, Default)]
pub struct BuildLogicPlugin;

impl Plugin for BuildLogicPlugin {
    fn apply(&self, project: &mut Project) -> ProjectResult {
        todo!()
    }
}
