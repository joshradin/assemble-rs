//! The javascript based builder

use crate::build_logic::plugin::script::languages::JavascriptLang;
use crate::build_logic::plugin::script::ScriptingLang;
use crate::builders::js::error::JavascriptError;
use std::fmt::{Debug, Formatter};

use crate::builders::js::types::Settings as JsSettings;
use crate::BuildConfigurator;
use assemble_core::prelude::{Assemble, AssembleAware, Settings, SettingsAware, StdResult};
use parking_lot::RwLock;

use crate::build_logic::{BuildLogic, NoOpBuildLogic};
use crate::builders::js::build_logic::JsBuildLogic;
use assemble_core::error::PayloadError;
use assemble_js::javascript;
use rquickjs::{Context, FromJs, IntoJs, Object, Runtime};
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;

pub mod build_logic;
pub mod error;
pub mod logging;
pub mod types;

/// A java script builder
pub struct JavascriptBuilder {
    runtime: Runtime,
}

impl Debug for JavascriptBuilder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JavascriptBuilder")
            .field("memory", &self.runtime.memory_usage())
            .finish()
    }
}

impl Default for JavascriptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl JavascriptBuilder {
    /// Creates a new java script builder
    pub fn new() -> Self {
        Self {
            runtime: Runtime::new().expect("could not create js runtime"),
        }
    }

    pub fn new_context(&self) -> Context {
        let context = Context::full(&self.runtime).expect("could not create js context");
        context.with(|ctx| {
            ctx.globals().init_def::<logging::Logger>().unwrap();
            ctx.globals()
                .init_def::<assemble_js::javascript::Bindings>()
                .unwrap();
        });
        context.with(|ctx| {
            ctx.eval::<(), _>(
                r#"
            const logger = new Logger();
            "#,
            )
            .expect("couldn't init logger");
        });
        context
    }

    pub fn configure_value<'a, T: for<'js> FromJs<'js>>(
        &self,
        var_name: &str,
        path: &Path,
        loads: impl IntoIterator<Item = &'a str>,
    ) -> Result<T, rquickjs::Error> {
        let context = self.new_context();
        context.with(|ctx| {
            for load in loads {
                trace!("loading:\n{}", load);
                ctx.eval(load)?;
            }

            trace!("evaluating script {:?}", path);
            ctx.eval_file(path)?;

            let result: T = ctx.eval(var_name)?;
            Ok(result)
        })
    }
}

impl BuildConfigurator for JavascriptBuilder {
    type Lang = JavascriptLang;
    type Err = JavascriptError;
    type BuildLogic<S: SettingsAware> = JsBuildLogic;

    fn get_build_logic<S: SettingsAware>(
        &self,
        settings: &S,
    ) -> StdResult<Self::BuildLogic<S>, PayloadError<Self::Err>> {
        Ok(JsBuildLogic::new(&self.runtime))
    }

    fn configure_settings<S: SettingsAware>(
        &self,
        setting: &mut S,
    ) -> StdResult<(), PayloadError<Self::Err>> {
        let settings_file = setting.with_settings(|p| p.settings_file().to_path_buf());
        let js_settings: JsSettings = self
            .configure_value(
                "settings",
                &settings_file,
                [
                    &format!(
                        r#"
                const current_dir = {:?};
                require("assemble");
                assemble.project_dir = {:?};
            "#,
                        setting.with_assemble(|s| s.current_dir().to_path_buf()),
                        setting.with_assemble(|s| s.project_dir())
                    ),
                    javascript::file_contents("settings.js").map_err(PayloadError::new)?,
                ],
            )
            .map_err(PayloadError::new)?;

        trace!("js settings: {:#?}", js_settings);
        setting.with_settings_mut(|s| {
            s.root_project_mut()
                .set_name(&js_settings.root_project.name);
            for desc in js_settings.root_project.children {
                s.add_project(desc.path, |pr| {
                    pr.set_name(desc.name);
                })
            }
        });
        Ok(())
    }

    fn discover<P: AsRef<Path>>(
        &self,
        path: P,
        assemble: &Arc<RwLock<Assemble>>,
    ) -> StdResult<Settings, PayloadError<Self::Err>> {
        let path = path.as_ref();

        for path in path.ancestors() {
            let script_path = path.join(Self::Lang::settings_script_name());
            trace!("searching for settings script at: {:?}", script_path);
            if script_path.exists() && script_path.is_file() {
                let mut settings = Settings::new(assemble, path.to_path_buf(), script_path);
                settings.set_build_file_name(JavascriptLang.build_script_name());
                trace!("found: {:?}", settings.settings_file());
                return Ok(settings);
            }
        }

        Err(JavascriptError::MissingSettingsFile.into())
    }
}
