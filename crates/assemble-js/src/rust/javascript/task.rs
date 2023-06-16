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
use rquickjs::{bind, Context, Ctx, Function, Object, Persistent, Value};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use once_cell::sync::OnceCell;

#[bind(public, object)]
#[quickjs(bare)]
mod js_task {
    use std::sync::Arc;
    use parking_lot::Mutex;
    use rquickjs::{Function, Object, Persistent};
    use rquickjs::class::Constructor;
    use assemble_core::Executable;
    use assemble_core::prelude::TaskId;
    use assemble_core::task::{HasTaskId, TaskHandle};
    use crate::javascript::id::IdObject;
    use crate::javascript::project::Project;
    use crate::javascript::task::JSTask;

    #[derive(Debug, Clone)]
    pub struct ExecutableJsTask {
        id: IdObject,
        object: Arc<Mutex<Persistent<Object<'static>>>>,
        actions: Vec<Persistent<Function<'static>>>,
    }

    impl ExecutableJsTask {
        pub fn new(cons: Object) -> Self {

            todo!()
        }

        pub fn doFirst(&self) {
            println!("doFirst")
        }

        pub fn toString(&self) -> String {
            todo!()
        }

        pub fn task_id(&self) -> IdObject {
            self.id.clone()
        }
    }
}

#[bind(public, object)]
#[quickjs(bare)]
mod task_provider {
    use std::cell::RefCell;
    use crate::javascript::task::JSTask;
    use crate::JsPluginExtension;
    use assemble_core::plugins::extensions::ExtensionAware;
    use assemble_core::project::shared::SharedProject;
    use log::{debug, error, info};
    use parking_lot::{Mutex, RwLock};
    use rquickjs::{Context, Ctx, Error, Exception, FromJs, Function, Object, Persistent, Value};
    use std::sync::Arc;
    use crate::javascript::project::{ProjectObj};
    use crate::javascript::task::js_task::ExecutableJsTask;
    use rquickjs::{IntoJs as _};
    use rquickjs::function::{AsArguments, IntoInput, This};
    use assemble_core::error::PayloadError;
    use assemble_core::exception::BuildException;
    use assemble_core::task::{HasTaskId, ResolveExecutable, ResolveInnerTask};

    #[derive(Debug)]
    pub struct JsTaskProvider {
        project: ProjectObj,
        ctor: Persistent<Function<'static>>,

        inner: RefCell<Option<ExecutableJsTask>>
    }

    impl JsTaskProvider {
        #[quickjs(skip)]
        pub fn new(project: SharedProject, inner: assemble_std::task::TaskHandle<JSTask>) -> Self {
            todo!()
        }

        pub fn configure<'js>(&self, ctx: Ctx<'js>, config: Function<'js>) {
            let mut exec = self.inner.borrow_mut();
            let persistent = self.ctor.clone();
            let obj = exec.get_or_insert_with(|| {

                let obj = persistent.restore(ctx).unwrap().call::<_, Object>(()).unwrap();
                ExecutableJsTask::new(obj)
            }).clone();

            let project_obj = self.project.clone();
            let task_id = obj.task_id();
            config.call::<_, ()>((obj, project_obj)).unwrap_or_else(|e| {
                match e {
                    Error::Exception => {
                        let exception = Exception::from_js(ctx, ctx.catch()).unwrap();
                        panic!("exception occurred while configuring task '{}': {}", task_id, exception)
                    }
                    e => {

                        panic!("error occurred while configuring task '{}': {}", task_id, e);
                    }
                }
            });

        }
    }
}

use crate::JsPluginExtension;
pub use task_provider::JsTaskProvider;
use crate::javascript::task::js_task::ExecutableJsTask;

#[derive(TaskIO)]
pub struct JSTask {
    task_obj: Arc<Mutex<OnceCell<ExecutableJsTask>>>,
}

impl JSTask {

}

impl Default for JSTask {
    fn default() -> Self {
        Self {
            task_obj: Arc::new(Mutex::new(OnceCell::new()))
        }
    }
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
