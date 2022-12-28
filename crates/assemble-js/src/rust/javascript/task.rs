//! provides task bindings

use assemble_core::__export::{CreateTask, InitializeTask, ProjectResult, TaskIO, TaskId};
use assemble_core::error::PayloadError;
use assemble_core::exception::BuildException;
use assemble_core::plugins::extensions::{Extension, ExtensionAware};
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::task::HasTaskId;
use assemble_core::{BuildResult, Executable, Project, Task};
use assemble_std::{CreateTask, TaskIO};
use log::{debug, info};
use parking_lot::Mutex;
use rquickjs::{bind, Context, Ctx, Function, Object, Persistent, This, Value};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

#[bind(public, object)]
#[quickjs(bare)]
mod tasks {
    use crate::javascript::task::JSTask;
    use crate::JsPluginExtension;
    use assemble_core::plugins::extensions::ExtensionAware;
    use assemble_core::project::shared::SharedProject;
    use log::info;
    use parking_lot::{Mutex, RwLock};
    use rquickjs::{Context, Ctx, Function, Persistent};
    use std::sync::Arc;

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
            self.inner
                .configure_with(|cfg, _| {
                    cfg.ctor.get_mut().insert(persis);
                    Ok(())
                })
                .expect("couldn't configure");
        }
    }
}

use crate::JsPluginExtension;
pub use tasks::TaskProvider;

#[derive(TaskIO, Default)]
pub struct JSTask {
    ctor: Mutex<Option<Persistent<Function<'static>>>>,
}

impl Debug for JSTask {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JSTask").finish()
    }
}

impl UpToDate for JSTask {}

impl InitializeTask for JSTask {
    fn initialize(task: &mut Executable<Self>, project: &Project) -> ProjectResult {
        Ok(())
    }
}

impl Task for JSTask {
    fn task_action(task: &mut Executable<Self>, project: &Project) -> BuildResult {
        let ext = project.extension::<JsPluginExtension>()?;
        // let cons = ext
        //     .container()
        //     .get(FuncKind::Ctor, &task.task_id())
        //     .unwrap()
        //     .get(0)
        //     .unwrap();
        // let js_actions = ext
        //     .container()
        //     .get(FuncKind::Action, &task.task_id())
        //     .map(|v| v.into_iter().collect::<Vec<_>>())
        //     .unwrap_or(vec![]);

        let mut engine = ext.engine().lock();
        let context = engine
            .new_context()
            .map_err(|e| PayloadError::<BuildException>::new(e))?;

        // context
        //     .with(|ctx| -> rquickjs::Result<()> {
        //         let cons = cons.lock().clone().restore(ctx)?;
        //         let task = cons.construct::<_, Object>((task.task_id().to_string(),))?;
        //         for action in js_actions {
        //             let restored = action.lock().clone().restore(ctx)?;
        //             restored.call::<_, ()>((This(()), task.clone()))?;
        //         }
        //
        //         let exec_method: Function = task.get("execute")?;
        //         exec_method.call((This(task),))?;
        //
        //         Ok(())
        //     })
        //     .map_err(|e| PayloadError::<BuildException>::new(e))
        Ok(())
    }
}

#[bind(public, object)]
#[quickjs(bare)]
mod executable_spec {
    use rquickjs::Object;

    #[derive(Debug)]
    pub struct ExecutableSpec {
        first: Vec<Object<'static>>,
    }
}
