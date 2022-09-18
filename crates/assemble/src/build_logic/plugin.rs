//! The build-logic plugin that the :build-logic project adds

use assemble_core::defaults::tasks::Empty;
use assemble_core::plugins::Plugin;
use assemble_core::prelude::*;
use assemble_core::task::ExecutableTask;

pub mod compilation;
pub mod script;

#[derive(Debug, Default)]
pub struct BuildLogicPlugin;

impl BuildLogicPlugin {
    /// The task name for the lifecycle task to compile all scripts
    pub const COMPILE_SCRIPTS_TASK: &'static str = "compile-scripts";
}

impl Plugin for BuildLogicPlugin {
    fn apply(&self, project: &mut Project) -> ProjectResult {
        project
            .task_container_mut()
            .register_task_with::<Empty, _>(Self::COMPILE_SCRIPTS_TASK, |t, _| {
                t.set_group("build");
                t.set_description("Lifecycle task to compile all build scripts");
                Ok(())
            })?;

        Ok(())
    }
}
