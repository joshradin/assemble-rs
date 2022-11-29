//! The project type

use std::ops::{Deref, DerefMut};
use rquickjs::{bind, class_def, Ctx, Function, IntoJs, Method, Object, Undefined, Value};
use assemble_std::project::SharedProject;
use crate::javascript::file_contents;
use crate::PhantomIntoJs;

#[derive(Debug)]
pub struct ProjectObj {
    shared: PhantomIntoJs<SharedProject>,
}
impl ProjectObj {
    pub fn new(project: SharedProject) -> Self {
        ProjectObj { shared: project }
    }
}

impl<'js> IntoJs<'js> for ProjectObj {
    fn into_js(self, ctx: Ctx<'js>) -> rquickjs::Result<Value<'js>> {
        let mut obj = Object::new(ctx)?;
        Ok(obj.into())
    }
}