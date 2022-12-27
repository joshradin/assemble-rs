//! provides Logger bindings.

use log::{log, Level};
pub use logging::Logger;
use regex::{Match, Regex};
use rquickjs::{bind, Ctx, FromJs, Function, Value};
use std::sync::{Arc, RwLock};
use std::{fmt, io};

fn js_log<'js, W: io::Write>(
    ctx: Ctx<'js>,
    level: Level,
    msg: &str,
    params: Vec<Value<'js>>,
    writer: Option<&mut W>,
) -> Result<(), rquickjs::Error> {
    let template = template_string(ctx, msg, params)?;
    match writer {
        None => {
            log!(level, "{}", template);
        }
        Some(writer) => {
            write!(writer, "{}", template);
        }
    }

    Ok(())
}

/// templates a string
pub fn template_string<'js>(
    ctx: Ctx<'js>,
    msg: &str,
    params: Vec<Value<'js>>,
) -> Result<String, rquickjs::Error> {
    let mut formatted = msg.to_string().clone();
    let pat = Regex::new(r"\{([^}.]+)?}").expect("should be valid regex");

    let params = params
        .into_iter()
        .map(|value| -> Result<_, rquickjs::Error> {
            let function = ctx.globals().get::<_, Function>("String")?;
            let result: String = function.call((value,))?;
            Ok(result)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut index = 0;
    let mut find = 0;
    while let Some(matched) = pat.captures(&formatted[find..].to_owned()) {
        let target = match matched.get(1) {
            None => {
                let out = params.get(index).ok_or(rquickjs::Error::NumArgs {
                    expected: 0..params.len(),
                    given: index,
                })?;
                index += 1;
                out
            }
            Some(val) => {
                let index: usize = val.as_str().parse().map_err(|e| {
                    rquickjs::Error::new_from_js_message("string", "int", "invalid integer")
                })?;

                params.get(index).ok_or(rquickjs::Error::NumArgs {
                    expected: 0..params.len(),
                    given: index,
                })?
            }
        };

        let whole = matched.get(0).unwrap();
        formatted.replace_range((whole.start() + find)..(whole.end() + find), target);
        find += whole.start() + target.len();
    }
    Ok(formatted)
}

#[bind(public, object)]
#[quickjs(bare)]
mod logging {
    use crate::javascript::logger::js_log;
    use crate::{Ctx, PhantomIntoJs};
    use assemble_std::utilities::LockedWriter;
    use log::{info, log, Level, LevelFilter};
    use parking_lot::Mutex;
    use regex::Regex;
    use rquickjs::{Object, Rest, Value};
    use std::io;
    use std::io::Write;
    use std::sync::Arc;

    #[derive(Clone, Default)]
    pub struct Logger {
        #[quickjs(hide)]
        pub(super) opt_writer: Option<LockedWriter<Box<dyn Write + Send>>>,
    }

    impl Logger {
        #[quickjs(constructor = false)]
        #[quickjs(skip)]
        pub fn new<W: io::Write + 'static + Send>(writer: W) -> Self {
            Self {
                opt_writer: Some(LockedWriter::new(Box::new(writer))),
            }
        }

        pub fn error<'js>(
            &mut self,
            context: Ctx<'js>,
            msg: String,
            params: Rest<Value<'js>>,
        ) -> Result<(), rquickjs::Error> {
            let writer = self.opt_writer.as_mut();
            js_log(context, Level::Error, &msg, params.into(), writer)
        }

        pub fn warn<'js>(
            &mut self,
            context: Ctx<'js>,
            msg: String,
            params: Rest<Value<'js>>,
        ) -> Result<(), rquickjs::Error> {
            let writer = self.opt_writer.as_mut();
            js_log(context, Level::Warn, &msg, params.into(), writer)
        }

        pub fn info<'js>(
            &mut self,
            context: Ctx<'js>,
            msg: String,
            params: Rest<Value<'js>>,
        ) -> Result<(), rquickjs::Error> {
            let writer = self.opt_writer.as_mut();
            js_log(context, Level::Info, &msg, params.into(), writer)
        }

        pub fn debug<'js>(
            &mut self,
            context: Ctx<'js>,
            msg: String,
            params: Rest<Value<'js>>,
        ) -> Result<(), rquickjs::Error> {
            let writer = self.opt_writer.as_mut();
            js_log(context, Level::Debug, &msg, params.into(), writer)
        }
        pub fn trace<'js>(
            &mut self,
            context: Ctx<'js>,
            msg: String,
            params: Rest<Value<'js>>,
        ) -> Result<(), rquickjs::Error> {
            let writer = self.opt_writer.as_mut();
            js_log(context, Level::Trace, &msg, params.into(), writer)
        }
    }

    //     // emits to logger.info
    //     function print(msg: string, ...params: any[]) {
    // logger.info(msg, params)
    // }
    //
    //     /// emits to logger.error
    //     function eprint(msg: string, ...params: any[]) {
    // logger.error(msg, params)
    // }
    pub fn print<'js>(
        context: Ctx<'js>,
        msg: String,
        params: Rest<Value<'js>>,
    ) -> Result<(), rquickjs::Error> {
        js_log::<&mut dyn Write>(context, Level::Info, &msg, params.into(), None)
    }

    pub fn eprint<'js>(
        context: Ctx<'js>,
        msg: String,
        params: Rest<Value<'js>>,
    ) -> Result<(), rquickjs::Error> {
        js_log::<&mut dyn Write>(context, Level::Error, &msg, params.into(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Engine;
    use assemble_std::utilities::LockedWriter;
    use rquickjs::{Context, Runtime, Undefined};
    use std::sync::{Arc, RwLock};

    #[test]
    fn dummy() {
        let _runtime = Runtime::new().expect("couldn't create runtime");
    }

    #[test]
    #[cfg(not(windows))]
    fn can_use_logger_from_js() {
        simple_logger::init();
        let expr = r##"
        logger.info("{}, {}", "Hello", "World!");
        "##;

        let runtime = Runtime::new().expect("couldn't create runtime");
        let context = Context::full(&runtime).unwrap();

        let mut buffer = LockedWriter::new(vec![]);

        context
            .with(|ctx| {
                ctx.globals().init_def::<Logging>()?;
                ctx.globals().set("logger", Logger::new(buffer.clone()))?;
                ctx.eval::<(), _>(expr)
            })
            .expect("couldn't execute expression");

        drop(context);
        drop(runtime);

        let unlocked_buffer = buffer.take().expect("other buffer still alive");
        let as_string = String::from_utf8(unlocked_buffer).unwrap();
        assert_eq!(as_string, "Hello, World!")
    }

    #[test]
    fn indexed_params() {
        simple_logger::init();
        let expr = r##"
        logger.info("{0}, {}, {}, {1}", "Hello", "World!");
        "##;

        let runtime = Runtime::new().expect("couldn't create runtime");
        let context = Context::full(&runtime).unwrap();

        let mut buffer = LockedWriter::new(vec![]);

        context
            .with(|ctx| {
                ctx.globals().init_def::<Logging>()?;
                ctx.globals().set("logger", Logger::new(buffer.clone()))?;
                ctx.eval::<(), _>(expr)
            })
            .expect("couldn't execute expression");

        drop(context);
        drop(runtime);

        let unlocked_buffer = buffer.take().expect("other buffer still alive");
        let as_string = String::from_utf8(unlocked_buffer).unwrap();
        assert_eq!(as_string, "Hello, Hello, World!, World!");
    }

    #[test]
    fn invalid_param() {
        simple_logger::init();
        let expr = r##"
        logger.info("{}, {}", 1);
        "##;

        let runtime = Runtime::new().expect("couldn't create runtime");
        let context = Context::full(&runtime).unwrap();

        let mut buffer = LockedWriter::new(vec![]);

        let result = context.with(|ctx| {
            ctx.globals().init_def::<Logging>()?;
            ctx.globals().set("logger", Logger::new(buffer.clone()))?;
            ctx.eval::<(), _>(expr)
        });

        assert!(result.is_err(), "result must be an error");
    }

    #[test]
    fn invalid_index() {
        simple_logger::init();
        let expr = r##"
        logger.info("{}, {2}", 1, 0);
        "##;

        let runtime = Runtime::new().expect("couldn't create runtime");
        let context = Context::full(&runtime).unwrap();

        let mut buffer = LockedWriter::new(vec![]);

        let result = context.with(|ctx| {
            ctx.globals().init_def::<Logging>()?;
            ctx.globals().set("logger", Logger::new(buffer.clone()))?;
            ctx.eval::<(), _>(expr)
        });

        assert!(result.is_err(), "result must be an error");
    }

    #[test]
    fn index_must_be_int() {
        simple_logger::init();
        let expr = r##"
        logger.info("{abd}", 1, 0);
        "##;

        let runtime = Runtime::new().expect("couldn't create runtime");
        let context = Context::full(&runtime).unwrap();

        let mut buffer = LockedWriter::new(vec![]);

        let result = context.with(|ctx| {
            ctx.globals().init_def::<Logging>()?;
            ctx.globals().set("logger", Logger::new(buffer.clone()))?;
            ctx.eval::<(), _>(expr)
        });

        assert!(result.is_err(), "result must be an error");
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    fn braces_in_args_ignored() {
        simple_logger::init();
        let expr = r##"
        logger.info("{}{}", "{2}", 0);
        "##;

        let runtime = Runtime::new().expect("couldn't create runtime");
        let context = Context::full(&runtime).unwrap();

        let mut buffer = LockedWriter::new(vec![]);

        context
            .with(|ctx| {
                ctx.globals().init_def::<Logging>()?;
                ctx.globals().set("logger", Logger::new(buffer.clone()))?;
                ctx.eval::<(), _>(expr)
            })
            .expect("shouldn't fail");

        drop(context);
        drop(runtime);

        let unlocked_buffer = buffer.take().expect("other buffer still alive");
        let as_string = String::from_utf8(unlocked_buffer).unwrap();
        assert_eq!(as_string, "{2}0");
    }
}
