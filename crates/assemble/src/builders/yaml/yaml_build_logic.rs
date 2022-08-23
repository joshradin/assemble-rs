use std::path::Path;
use assemble_core::prelude::SharedProject;
use std::fs::File;
use assemble_core::__export::ProjectError;
use assemble_core::Project;
use itertools::Itertools;
use heck::ToLowerCamelCase;
use assemble_core::defaults::tasks::Empty;
use assemble_core::task::task_container::FindTask;
use crate::build_logic::plugin::BuildLogicPlugin;
use crate::build_logic::plugin::script::languages::YamlLang;
use crate::builders::ProjectProperties;
use crate::builders::yaml::{SETTINGS_FILE_NAME, YamlBuilderError};
use crate::builders::yaml::settings::Settings;
use crate::BuildSettings;

/// Create the `:build-logic` project from a yaml settings files
pub struct YamlBuilder;

impl YamlBuilder {
    /// Creates the build-logic project with the yaml plugin
    fn create_build_logic(
        &self,
        settings: &Settings,
        root_dir: &Path,
    ) -> Result<SharedProject, ProjectError> {
        trace!("settings: {:#?}", settings);

        let build_scripts = settings.projects(root_dir);
        debug!("build_scripts = {:#?}", build_scripts);

        let mut shared = Project::in_dir_with_id(root_dir.join("build-logic"), "build-logic")?;
        shared.apply_plugin::<BuildLogicPlugin>()?;

        for project in &build_scripts {
            let id = self.compile_project_script_task_id(project.name());
            let task = shared.tasks().register_task_with::<Empty, _>(&id, |name, p| {
                Ok(())
            })?;
            shared.get_typed_task::<Empty, _>(BuildLogicPlugin::COMPILE_SCRIPTS_TASK)?
                .configure_with(|t, _| {
                    t.depends_on(task);
                    Ok(())
                })?;
        }

        Ok(shared)
    }

    fn compile_project_script_task_id(&self, project_name: &str) -> String {
        ["compile", project_name, "build", "script"]
            .into_iter()
            .join("-")
            .to_lower_camel_case()
    }
}

impl BuildSettings for YamlBuilder {
    type Lang = YamlLang;
    type Err = YamlBuilderError;

    fn open<P: AsRef<Path>>(
        &self,
        path: P,
        properties: &ProjectProperties,
    ) -> Result<SharedProject, Self::Err> {
        let settings_file_name = properties
            .get("settings.file")
            .map(|s| s.as_ref())
            .flatten()
            .cloned()
            .unwrap_or(String::from(SETTINGS_FILE_NAME));

        let joined = path.as_ref().join(settings_file_name);
        let file = File::open(joined)
            .map_err(|_| YamlBuilderError::MissingSettingsFile(path.as_ref().to_path_buf()))?;
        let settings: Settings = serde_yaml::from_reader(file)?;
        Ok(self.create_build_logic(&settings, path.as_ref())?)
    }

    fn discover<P: AsRef<Path>>(
        &self,
        path: P,
        properties: &ProjectProperties,
    ) -> Result<SharedProject, Self::Err> {
        let path = path.as_ref();
        for ancestor in path.ancestors() {
            match self.open(ancestor, properties) {
                Ok(p) => {
                    return Ok(p);
                }
                Err(YamlBuilderError::MissingSettingsFile(_)) => {
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            }

        }
        Err(YamlBuilderError::MissingSettingsFile(path.to_path_buf()))
    }
}
