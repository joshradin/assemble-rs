use crate::build_logic::BuildLogic;
use crate::builders::js::error::JavascriptError;
use assemble_core::error::PayloadError;
use assemble_core::prelude::{SettingsAware, SharedProject};
use assemble_js_plugin::{Engine, javascript};
use rquickjs::Runtime;
use assemble_core::logging::LOGGING_CONTROL;
use assemble_core::project::GetProjectId;

/// The js build logic engine
#[derive(Debug)]
pub struct JsBuildLogic {
    engine: Engine,
}

impl JsBuildLogic {
    pub fn new(runtime: &Runtime) -> Self {
        Self {
            engine: Engine::with_runtime(runtime),
        }
    }
}

impl<S: SettingsAware> BuildLogic<S> for JsBuildLogic {
    type Err = JavascriptError;

    fn configure(
        &mut self,
        settings: &S,
        project: &SharedProject,
    ) -> Result<(), Self::Err> {
        LOGGING_CONTROL.in_project(
            project.project_id()
        );
        trace!("configuring project {}", project);

        let file = settings.with_settings(|s| {
            let project_dir = project.with(|p| p.project_dir());
            s.find_project(project_dir)
             .and_then(|desc| desc.build_file())
             .map(|p| p.to_path_buf())
        }).expect("build file must be set, even if it doesn't exist");

        trace!("found potential build file: {:?}", file);
        if file.try_exists()? {
            info!("creating delegate of project {} from engine {:?}", project, self.engine);
            let mut delegating = self
                .engine
                .delegate_to("project", javascript::project::ProjectObj::new(project.clone()))?;

            info!("build file exists ({:?}), evaluating...", file);
            delegating.eval_file_once(&file)
                .map_err(|e| {
                    JavascriptError::RQuickJsErrorWithFile(e, file)
                })?;
        } else {
            warn!("no build file found for project {} at {:?}", project, file);
        }

        LOGGING_CONTROL.reset();

        project.with(|p| -> Result<(), Self::Err> {
            for sub in p.subprojects() {
                self.configure(settings, sub)?;
            }
            Ok(())
        })?;

        Ok(())
    }
}
