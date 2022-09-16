use crate::build_logic::plugin::script::languages::YamlLang;
use crate::build_logic::plugin::BuildLogicPlugin;
use crate::builders::compile_project::CompileProject;
use crate::builders::create_cargo_file::CreateCargoToml;
use crate::builders::yaml::compiler::YamlCompiler;
use crate::builders::yaml::settings::Settings;
use crate::builders::yaml::{YamlBuilderError, SETTINGS_FILE_NAME};
use crate::builders::{CompileBuildScript, ProjectProperties};
use crate::BuildSettings;
use assemble_core::project::error::ProjectError;
use assemble_core::cache::AssembleCache;
use assemble_core::cryptography::{hash_sha256, Sha256};
use assemble_core::defaults::tasks::Empty;
use assemble_core::prelude::{Provider, SharedProject};
use assemble_core::task::task_container::FindTask;
use assemble_core::task::{HasTaskId, TaskProvider};
use assemble_core::Project;
use heck::ToLowerCamelCase;
use itertools::Itertools;
use std::fs::{create_dir_all, File};
use std::ops::Deref;
use std::path::Path;
use assemble_core::project::ProjectResult;
use assemble_rust::plugin::RustBasePlugin;

/// Create the `:build-logic` project from a yaml settings files
pub struct YamlBuilder;

impl YamlBuilder {
    /// Creates the build-logic project with the yaml plugin
    fn create_build_logic(
        &self,
        settings: &Settings,
        root_dir: &Path,
    ) -> ProjectResult<SharedProject> {
        trace!("settings: {:#?}", settings);

        let build_scripts = settings.projects(root_dir);
        debug!("build_scripts = {:#?}", build_scripts);

        let mut shared = Project::in_dir_with_id(root_dir.join("build-logic"), "build-logic")?;
        shared.apply_plugin::<BuildLogicPlugin>()?;

        let sha = hash_sha256(&format!("{root_dir:?}")).to_string();
        let build_logic_path = AssembleCache::default().join("projects").join(sha);
        create_dir_all(&build_logic_path)?;

        let mut script_tasks = vec![];
        for project in &build_scripts {
            let id = self.compile_project_script_task_id(project.name());
            let path = project.path().to_path_buf();
            let output_path = build_logic_path.join(format!("{}-compiled.rs", project.name()));
            let task = shared
                .tasks()
                .register_task_with::<CompileBuildScript<YamlLang, YamlCompiler>, _>(
                    &id,
                    |compile_task, p| {
                        compile_task.script_path.set(path)?;
                        compile_task.output_file.set(output_path)?;
                        Ok(())
                    },
                )?;
            let task_clone = task.clone();
            shared
                .get_typed_task::<Empty, _>(BuildLogicPlugin::COMPILE_SCRIPTS_TASK)?
                .configure_with(|t, _| {
                    t.depends_on(task);
                    Ok(())
                })?;
            script_tasks.push(task_clone);
        }

        let cargo_toml = build_logic_path.join("Cargo.toml");
        let script_tasks_clone = script_tasks.clone();

        let cargo_toml_task = shared.tasks().register_task_with::<CreateCargoToml, _>(
            "createCargoToml",
            move |task, project| {
                task.depends_on(script_tasks.clone());
                let scripts: Vec<TaskProvider<_, _, _>> = script_tasks
                    .into_iter()
                    .map(|t| t.provides(|t| t.compiled_script().get()))
                    .collect();
                for script_provider in scripts {
                    let mut prop = task.task_id().prop(&format!("scripts-{}", task.scripts.len()))?;
                    prop.set_with(script_provider)?;
                    task.scripts.push(prop);
                }
                task.config_path.set(cargo_toml)?;
                Ok(())
            },
        )?;

        shared.apply_plugin::<RustBasePlugin>()?;

        let compile = shared.tasks().register_task_with::<CompileProject, _>(
            "compileBuildLogicProject",
            move |task, project| {
                task.depends_on(cargo_toml_task);
                task.depends_on(script_tasks_clone);
                task.depends_on("install-default-toolchain");
                task.source(&build_logic_path);
                task.project_dir.set(build_logic_path)?;
                Ok(())
            },
        )?;

        let mut compile_script_lifecycle = shared
            .task_container()
            .get_task(BuildLogicPlugin::COMPILE_SCRIPTS_TASK)?
            .as_type::<Empty>()
            .unwrap();

        compile_script_lifecycle.configure_with(|task, _| {
            task.depends_on(compile);
            Ok(())
        })?;

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
