use rquickjs::{Context, Ctx, IntoJs, Object, ObjectDef, Runtime};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};

pub mod javascript;

/// Provides an engine for executing scripts in
pub struct Engine {
    libs: Vec<String>,
    bindings: Vec<Box<dyn for<'js> FnMut(Ctx<'js>, &Object<'js>) -> rquickjs::Result<()>>>,
    runtime: Runtime,
}

impl Debug for Engine {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine")
            .field("libs", &self.libs)
            .field("bindings", &self.bindings.len())
            .finish()
    }
}

impl Engine {
    /// Creates a new engine
    pub fn new() -> Self {
        Self {
            libs: vec![],
            bindings: vec![],
            runtime: Runtime::new().expect("a js runtime"),
        }
        .with_bindings::<javascript::Bindings>()
    }

    /// Adds libraries
    pub fn with_libs<S: AsRef<str>, I: IntoIterator<Item = S>>(mut self, iter: I) -> Self {
        self.using_libs(iter);
        self
    }

    /// Use libraries
    pub fn using_libs<S: AsRef<str>, I: IntoIterator<Item = S>>(&mut self, iter: I) {
        self.libs
            .extend(iter.into_iter().map(|s| s.as_ref().to_string()));
    }

    pub fn with_declaration<
        K: AsRef<str> + Clone + 'static,
        V: for<'a> IntoJs<'a> + 'static + Clone,
    >(
        mut self,
        key: K,
        value: V,
    ) -> Self {
        self.using_declaration(key, value);
        self
    }

    pub fn using_declaration<
        K: AsRef<str> + Clone + 'static,
        V: for<'a> IntoJs<'a> + 'static + Clone,
    >(
        &mut self,
        key: K,
        value: V,
    ) {
        let cls = move |ctx: Ctx, object: &Object| -> rquickjs::Result<()> {
            ctx.globals().set(key.clone().as_ref(), value.clone())?;
            Ok(())
        };
        self.bindings.push(Box::new(cls));
    }

    pub fn with_bindings<T: ObjectDef + 'static>(mut self) -> Self {
        self.using_bindings::<T>();
        self
    }

    /// Adds binding object
    pub fn using_bindings<T: ObjectDef + 'static>(&mut self) {
        let func = Box::new(T::init);
        self.bindings.push(func);
    }

    /// Creates a new context
    pub fn new_context(&mut self) -> rquickjs::Result<Context> {
        let mut context = Context::full(&self.runtime)?;
        context.with(|ctx| -> rquickjs::Result<()> {
            for binding in &mut self.bindings {
                binding(ctx.clone(), &ctx.globals())?;
            }

            Ok(())
        })?;
        Ok(context)
    }
}
