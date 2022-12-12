//! The project type

use crate::javascript::file_contents;
use crate::PhantomIntoJs;
use assemble_std::project::SharedProject;
use rquickjs::{bind, class_def, Ctx, Function, IntoJs, Method, Object, Undefined, Value};
use std::ops::{Deref, DerefMut};

#[bind(public, object)]
#[quickjs(bare)]
mod project {
    use crate::javascript::task::{JSTask, TaskProvider};
    use crate::PhantomIntoJs;
    use assemble_std::prelude::{ProjectId, SharedProject};
    use log::{info, trace};
    use rquickjs::{Constructor, Ctx, Function, Value};

    #[derive(Debug)]
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

        pub fn register<'js>(&self, ctx: Ctx<'js>, name: String, create: Function) -> TaskProvider {
            trace!(
                "attempting to register task {} with create function: {:?}",
                name,
                create
            );
            let handle = self
                .shared
                .tasks()
                .with_mut(|tc| tc.register_task_with::<JSTask, _>(&name, |s, _| Ok(())))
                .expect("invalid handle");
            TaskProvider {
                project: self.shared.clone(),
                inner: handle,
            }
        }
    }
}

pub use project::ProjectObj;
