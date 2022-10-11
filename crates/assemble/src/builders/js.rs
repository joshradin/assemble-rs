//! The javascript based builder

use crate::build_logic::plugin::script::languages::JavascriptLang;
use crate::build_logic::plugin::script::ScriptingLang;
use crate::builders::js::error::JavascriptError;

use crate::builders::js::types::Settings as JsSettings;
use crate::{BuildConfigurator, BuildLogic};
use assemble_core::prelude::{Assemble, Settings, SettingsAware};
use parking_lot::RwLock;

use rquickjs::{Context, FromJs, IntoJs, Runtime};
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

pub mod error;
pub mod logging;
pub mod types;

pub struct JavascriptBuilder {
    runtime: Pin<Box<Runtime>>,
    context: Context,
}

impl JavascriptBuilder {
    /// Creates a new java script builder
    pub fn new() -> Self {
        let runtime = Pin::new(Box::new(
            Runtime::new().expect("couldn't create a js runtime."),
        ));
        let ctxt = Context::full(&*runtime).expect("could not create context");
        ctxt.with(|ctx| {
            // bind_logging(ctx).expect("could not init logging");
            ctx.globals().init_def::<logging::Logger>().unwrap();
            ctx.eval::<(), _>(include_str!("js/settings.js")).unwrap();
        });
        ctxt.with(|ctx| {
            // bind_logging(ctx).expect("could not init logging");
            ctx.eval::<(), _>(
                r"
            const logger = new Logger();
            ",
            )
            .expect("couldn't init logger");
        });
        Self {
            runtime,
            context: ctxt,
        }
    }

    pub fn configure_value<T: for<'js> IntoJs<'js> + for<'js> FromJs<'js> + Clone>(
        &self,
        var_name: &str,
        js_type: &str,
        value: &mut T,
        path: &Path,
    ) -> Result<(), rquickjs::Error> {
        self.context.with(|ctx| {
            let as_js_value = value.clone().into_js(ctx)?;
            ctx.globals().set("__temp", as_js_value)?;
            ctx.eval(format!("const {} = new {}(__temp);", var_name, js_type))?;
            ctx.eval_file(path)?;

            let value_after = ctx.eval::<T, _>(var_name)?;
            // let value_converted = T::from_js(ctx, value_after)?;
            *value = value_after;

            Ok(())
        })
    }
}

impl BuildConfigurator for JavascriptBuilder {
    type Lang = JavascriptLang;
    type Err = JavascriptError;

    fn get_build_logic<S: SettingsAware>(&self, _settings: &S) -> Result<BuildLogic, Self::Err> {
        todo!()
    }

    fn configure_settings<S: SettingsAware>(&self, setting: &mut S) -> Result<(), Self::Err> {
        let settings_file = setting.with_settings(|p| p.settings_file().to_path_buf());
        let mut js_settings = JsSettings::default();

        self.configure_value("settings", "Settings", &mut js_settings, &settings_file)?;

        debug!("js settings: {:#?}", js_settings);
        todo!("convert js settings to settings");
        Ok(())
    }

    fn discover<P: AsRef<Path>>(
        &self,
        path: P,
        assemble: &Arc<RwLock<Assemble>>,
    ) -> Result<Settings, Self::Err> {
        let path = path.as_ref();

        for path in path.ancestors() {
            let script_path = path.join(Self::Lang::settings_script_name());
            info!("searching for settings script at: {:?}", script_path);
            if script_path.exists() && script_path.is_file() {
                let mut settings = Settings::new(assemble, path.to_path_buf(), script_path);
                settings.set_build_file_name(JavascriptLang.build_script_name());
                info!("found: {:?}", settings.settings_file());
                return Ok(settings);
            }
        }

        Err(JavascriptError::MissingSettingsFile)
    }
}
