use rquickjs::{Context, Ctx, FromJs, IntoJs, Object, ObjectDef, Runtime};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;

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

    pub fn delegate_to<'js, V: IntoJs<'js> + FromJs<'js>>(
        &mut self,
        key: &str,
        value: V,
    ) -> rquickjs::Result<Delegating<'js, V>> {
        self.new_context().map(|context| Delegating {
            key: key.to_string(),
            value,
            context,
            _lt: PhantomData,
        })
    }
}

pub struct Delegating<'js, V: IntoJs<'js> + FromJs<'js>> {
    key: String,
    value: V,
    context: Context,
    _lt: PhantomData<&'js ()>,
}

impl<'js, V: IntoJs<'js> + FromJs<'js>> Delegating<'js, V> {
    pub fn eval<S: Into<Vec<u8>>>(self, evaluate: S) -> rquickjs::Result<Self> {
        let Delegating {
            key,
            value,
            context,
            _lt,
        } = self;

        let value = context.with(move |ctx| -> rquickjs::Result<V> {
            ctx.globals().set(&*key, value)?;
            drop(ctx.eval(evaluate)?);
            let ret = ctx.globals().get(&*key)?;
            Ok(ret)
        })?;

        Ok(Self {
            key,
            value,
            context,
            _lt,
        })
    }

    pub fn finish(self) -> V {
        self.value
    }
}
