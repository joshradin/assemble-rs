use log::{debug, info};
use rquickjs::{Context, Ctx, FromJs, IntoJs, Object, ObjectDef, Runtime, Undefined, Value};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::path::Path;
use std::ops::{Deref, DerefMut};

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
    pub fn with_runtime(runtime: &Runtime) -> Self {
        Self {
            libs: vec![],
            bindings: vec![],
            runtime: runtime.clone(),
        }
        .with_bindings::<javascript::Bindings>()
        .with_bindings::<javascript::Logging>()
            .with_declaration("logger", javascript::logging::Logger::new())

    }

    /// Creates a new engine
    pub fn new() -> Self {
        Self::with_runtime(&Runtime::new().expect("a js runtime"))
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
            info!("attempting to set global {}", key.as_ref());
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
        let mut context = Context::full(&Runtime::new()?)?;
        context.with(|ctx| -> rquickjs::Result<()> {
            for binding in &mut self.bindings {
                debug!("executing binding");
                binding(ctx.clone(), &ctx.globals())?;
            }

            Ok(())
        })?;
        Ok(context)
    }

    pub fn delegate_to<V>(&mut self, key: &str, value: V) -> rquickjs::Result<Delegating<V>>
    where
        for<'js> V: IntoJs<'js>
    {
        self.new_context().map(|context| Delegating {
            key: key.to_string(),
            value,
            context,
        })
    }
}

pub struct Delegating<V>
where
    for<'js> V: IntoJs<'js>,
{
    key: String,
    value: V,
    context: Context,
}

impl<V> Delegating<V>
where
    for<'js> V: IntoJs<'js>,
{
    pub fn new(context: Context, key: &str, value: V) -> Self {
        Self {
            key: key.to_string(),
            value,
            context,
        }
    }

    pub fn eval_file_once<P: AsRef<Path>>(self, file: P) -> rquickjs::Result<()> {
        let opened = std::fs::read(file)?;
        self.eval_once(opened)
    }

    pub fn eval_once<S: Into<Vec<u8>>, O : for<'js> FromJs<'js>>(self, evaluate: S) -> rquickjs::Result<O> {
        let orig = self.value;
        let key = self.key;
        let ret = self.context.with(|ctx: Ctx| -> rquickjs::Result<_> {
            ctx.globals().set(&*key, orig)?;
            let bytes = evaluate.into();
            let ret: O = ctx.eval(bytes)?;
            Ok(ret)
        })?;

        Ok(ret)
    }
}

impl<V: Clone> Delegating<V>
    where
            for<'js> V: IntoJs<'js> + FromJs<'js>,
{
    pub fn eval_file<P: AsRef<Path>>(&mut self, file: P) -> rquickjs::Result<()> {
        let opened = std::fs::read(file)?;
        self.eval(opened)
    }
    pub fn eval<S: Into<Vec<u8>>, O>(&mut self, evaluate: S) -> rquickjs::Result<O>
    where
        for<'js> O: FromJs<'js>,
    {
        let orig = self.value.clone();
        let key = self.key.clone();
        let (value, ret) = self.context.with(|ctx: Ctx| -> rquickjs::Result<_> {
            ctx.globals().set(&*key, orig)?;
            let bytes = evaluate.into();
            let ret: O = ctx.eval(bytes)?;
            let value: V = ctx.globals().get(&*key)?;
            Ok((value, ret))
        })?;

        self.value = value;

        Ok(ret)
    }

    pub fn finish(self) -> V {
        self.value
    }
}

#[macro_export]
macro_rules! delegate_to {
    ($context:expr, $var:ident) => {
        $crate::Delegating::new($context, stringify!($var), $var)
    };
}

#[cfg(test)]
mod tests {
    use crate::Delegating;
    use rquickjs::{Context, Runtime};

    #[test]
    fn can_delegate() -> rquickjs::Result<()> {
        let ref runtime = Runtime::new()?;
        let number = 0;
        let mut delegate = delegate_to!(Context::full(runtime)?, number);
        delegate.eval("number = 10;")?;

        assert_eq!(delegate.finish(), 10);

        Ok(())
    }


}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct PhantomIntoJs<T>(pub T);

impl<T> Deref for PhantomIntoJs<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl <T> DerefMut for PhantomIntoJs<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'js, T> IntoJs<'js> for PhantomIntoJs<T> {
    fn into_js(self, ctx: Ctx<'js>) -> rquickjs::Result<Value<'js>> {
        Ok(Undefined.into_value(ctx))
    }
}
