//! The project type

use crate::javascript::file_contents;
use crate::PhantomIntoJs;
use assemble_core::project::shared::SharedProject;
use rquickjs::{bind, class_def, Ctx, FromJs, Function, IntoJs, Object, Undefined, Value};
use std::ops::{Deref, DerefMut};

#[derive(Debug, FromJs)]
#[quickjs(untagged)]
pub enum PluginOptions {
    VersionOnly(String),
    Options {
        version: String
    }
}

#[bind(public, object)]
mod project {
    use std::collections::HashMap;
    use indexmap::IndexMap;
    use crate::javascript::task::{JSTask, JsTaskProvider};
    use crate::{JsPluginExtension, PhantomIntoJs};
    use assemble_core::plugins::extensions::ExtensionAware;
    use assemble_core::project::shared::SharedProject;
    use assemble_std::prelude::ProjectId;
    use log::{info, trace};
    use rquickjs::{Ctx, Function, Object, Persistent, Value};
    use rquickjs::function::This;
    use assemble_core::task::TaskProvider;
    use crate::javascript::project::PluginOptions;

    #[derive(Debug, Clone)]
    pub struct ProjectObj {
        #[quickjs(skip)]
        shared: SharedProject,
    }


    impl ProjectObj {
        #[quickjs(skip)]
        pub fn new(project: SharedProject) -> Self {
            ProjectObj {
                shared: project.into(),
            }
        }

        pub fn toString(&self) -> String {
            self.shared.to_string()
        }

        pub fn plugins(&self, plugins: IndexMap<String, PluginOptions>) {
            info!("plugins to apply: {plugins:#?}")
        }

        pub fn register<'js>(
            &self,
            ctx: Ctx<'js>,
            name: String,
            create: Function<'js>,
        ) -> JsTaskProvider {
            trace!(
                "attempting to register task {} with create function: {:?}",
                name,
                create
            );
            let mut handle = self
                .shared
                .tasks()
                .with_mut(|tc| tc.register_task_with::<JSTask, _>(&name, |s, _| Ok(())))
                .expect("invalid handle");

            let task_object: Object = create.construct(()).expect("could not create");
            let persisted=Persistent::save(ctx, task_object);
            handle.configure_with(|task, _| {
                Ok(())
            }).unwrap();


            // JsTaskProvider::new(
            //     self.shared.clone(),
            //     handle
            // )
            todo!()
        }
    }

}

pub use project::*;

#[bind(public, object)]
#[quickjs(bare)]
pub mod project_script {
    use indexmap::IndexMap;
    use rquickjs::{Ctx, Function, Object};
    use rquickjs::function::{Method, This};
    use crate::javascript::project::PluginOptions;


    fn plugins<'js>(ctx: Ctx<'js>, plugins: Object<'js>) {
        let project: Object = ctx.globals().get("project").expect("project not defined");
        let plugins_func = project.get::<_, Function>("plugins").expect("plugins not defined on project");
        plugins_func.call::<_, ()>((This(project), plugins)).expect("couldn't run plugins");
    }
}
