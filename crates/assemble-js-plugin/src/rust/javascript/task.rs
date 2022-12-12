//! provides task bindings

use assemble_core::__export::{CreateTask, InitializeTask, ProjectResult, TaskIO, TaskId};
use assemble_core::plugins::extensions::Extension;
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::{BuildResult, Executable, Project, Task};
use assemble_std::{CreateTask, TaskIO};
use parking_lot::Mutex;
use rquickjs::{bind, Context, Ctx, Function, Persistent};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

#[bind(public, object)]
#[quickjs(bare)]
mod tasks {
    use crate::javascript::task::{JSTask, TaskActionContainer};
    use assemble_core::plugins::extensions::ExtensionAware;
    use assemble_core::prelude::SharedProject;
    use log::info;
    use parking_lot::{Mutex, RwLock};
    use rquickjs::{Context, Ctx, Function, Persistent};
    use std::sync::Arc;
    use crate::JsPluginExtension;

    #[derive(Debug)]
    pub struct TaskProvider {
        #[quickjs(skip)]
        pub project: SharedProject,
        #[quickjs(skip)]
        pub inner: assemble_std::task::TaskHandle<JSTask>,
    }

    impl TaskProvider {
        pub fn configure<'js>(&mut self, ctx: Ctx<'js>, config: Function<'js>) {
            let persis: Persistent<Function<'static>> = Persistent::save(ctx, config);
            self.project.with_mut(|s| {
                let actions = s
                    .extension_mut::<JsPluginExtension>()
                    .expect("js plugin not added");
                actions.container_mut().insert(self.inner.id(), persis);
            });
        }
    }
}
pub use tasks::TaskProvider;

#[derive(TaskIO)]
pub struct JSTask {}

impl Debug for JSTask {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JSTask").finish()
    }
}

impl CreateTask for JSTask {
    fn new(using_id: &TaskId, project: &Project) -> ProjectResult<Self> {
        Ok(JSTask { })
    }
}

impl UpToDate for JSTask {}

impl InitializeTask for JSTask {}

impl Task for JSTask {
    fn task_action(_task: &mut Executable<Self>, _project: &Project) -> BuildResult {
        todo!()
    }
}

#[derive(Debug)]
pub struct TaskActionContainer {
    map: HashMap<TaskId, Vec<Mutex<Persistent<Function<'static>>>>>,
}

impl TaskActionContainer {
    pub fn new() -> Self {
        Self { map: HashMap::new()}
    }

    pub fn insert(&mut self, id: &TaskId, func: Persistent<Function<'static>>) {
        self.map
            .entry(id.clone())
            .or_default()
            .push(Mutex::new(func));
    }
}
