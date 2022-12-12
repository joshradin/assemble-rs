use crate::build_logic::BuildLogic;
use crate::builders::js::error::JavascriptError;
use assemble_core::error::PayloadError;
use assemble_core::logging::LOGGING_CONTROL;
use assemble_core::plugins::extensions::ExtensionAware;
use assemble_core::prelude::{SettingsAware, SharedProject};
use assemble_core::project::GetProjectId;
use assemble_js_plugin::{javascript, Delegating, Engine, JsPlugin, JsPluginExtension};
use rquickjs::Runtime;

/// The js build logic engine
#[derive(Debug)]
pub struct JsBuildLogic {
    engine: Engine,
}

impl JsBuildLogic {
    pub fn new(runtime: &Runtime) -> Self {
        Self {
            engine: Engine::with_runtime(runtime).with_bindings::<javascript::project::Project>(),
        }
    }
}

impl<S: SettingsAware> BuildLogic<S> for JsBuildLogic {
    type Err = JavascriptError;

    fn configure(
        &mut self,
        settings: &S,
        project: &SharedProject,
    ) -> Result<(), PayloadError<Self::Err>> {
        LOGGING_CONTROL.in_project(project.project_id());
        trace!("configuring project {}", project);
        project
            .apply_plugin::<JsPlugin>()
            .expect("couldn't add js plugin");

        let file = settings
            .with_settings(|s| {
                let project_dir = project.with(|p| p.project_dir());
                s.find_project(project_dir)
                    .and_then(|desc| desc.build_file())
                    .map(|p| p.to_path_buf())
            })
            .expect("build file must be set, even if it doesn't exist");

        trace!("found potential build file: {:?}", file);
        if file.try_exists().map_err(|e| JavascriptError::from(e))? {
            trace!(
                "creating delegate of project {} from engine {:?}",
                project,
                self.engine
            );

            let delegating = project.with(|p| -> Result<Delegating<_>, Self::Err> {
                let js_ext = p.extension::<JsPluginExtension>().unwrap();
                let mut engine = js_ext.engine().lock();
                engine.using_bindings::<javascript::project::Project>();
                Ok(engine.delegate_to(
                    "project",
                    javascript::project::ProjectObj::new(project.clone()),
                )?)
            })?;

            trace!("build file exists ({:?}), evaluating...", file);
            delegating
                .eval_file_once(&file)
                .map_err(|e| JavascriptError::RQuickJsErrorWithFile(e, file))?;
        } else {
            warn!("no build file found for project {} at {:?}", project, file);
        }

        LOGGING_CONTROL.reset();

        project.with(|p| -> Result<(), PayloadError<Self::Err>> {
            for sub in p.subprojects() {
                self.configure(settings, sub)?;
            }
            Ok(())
        })?;

        Ok(())
    }
}
