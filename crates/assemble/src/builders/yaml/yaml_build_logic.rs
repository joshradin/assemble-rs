use crate::assemble_core::lazy_evaluation::ProviderExt;
use crate::build_logic::plugin::script::languages::YamlLang;
use crate::build_logic::plugin::BuildLogicPlugin;
use crate::builders::compile_project::CompileProject;
use crate::builders::create_cargo_file::CreateCargoToml;
use crate::builders::create_lib_file::CreateLibRs;
use crate::builders::patch_cargo::PatchCargoToml;
use crate::builders::yaml::compiler::YamlCompiler;
use crate::builders::yaml::settings::Settings;
use crate::builders::yaml::{YamlBuilderError, SETTINGS_FILE_NAME};
use crate::builders::{CompileBuildScript, ProjectProperties};
use crate::{BuildLogicExtension, BuildSettings};
use assemble_core::cache::AssembleCache;
use assemble_core::cryptography::hash_sha256;
use assemble_core::defaults::tasks::Empty;
use assemble_core::file_collection::FileSet;

use assemble_core::lazy_evaluation::anonymous::AnonymousProvider;

use assemble_core::plugins::extensions::ExtensionAware;
use assemble_core::prelude::{Provider, SharedProject};

use assemble_core::project::ProjectResult;
use assemble_core::task::task_container::FindTask;
use assemble_core::task::TaskProvider;
use assemble_core::Project;

use assemble_rust::plugin::RustBasePlugin;

use itertools::Itertools;
use std::fs::{create_dir_all, File};

use std::path::Path;

/// Create the `:build-logic` project from a yaml settings files
pub struct YamlBuilder;

static CREATE_CARGO_TOML: &str = "create-cargo-toml";
static CREATE_LIB_RS: &str = "create-lib-rs";
static COMPILE_BUILD_LOGIC_PROJECT: &str = "compile-build-logic-project";

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

        let actual = root_dir.join("build-logic");

        let shared = Project::in_dir_with_id(
            if actual.exists() {
                actual
            } else {
                root_dir.to_path_buf()
            },
            "build-logic",
        )?;
        shared.apply_plugin::<BuildLogicPlugin>()?;

        let sha = hash_sha256(&format!("{root_dir:?}")).to_string();
        let build_logic_path = AssembleCache::default().join("projects").join(sha);
        create_dir_all(&build_logic_path)?;

        let mut script_tasks = vec![];
        for project in &build_scripts {
            let project_name = project.project_id()?;
            let id = self.compile_project_script_task_id(project.name());
            let path = project.path().to_path_buf();
            let output_path = build_logic_path.join(format!("{}-compiled.rs", project.name()));
            let task = shared
                .tasks()
                .register_task_with::<CompileBuildScript<YamlLang, YamlCompiler>, _>(
                    &id.clone(),
                    move |compile_task, _p| {
                        compile_task.project_id.set(project_name)?;
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

        let build_cargo_toml = shared.with(|p| p.build_dir().map(|d| d.join("Cargo.toml")));
        let script_tasks_clone = script_tasks.clone();

        let cargo_toml_task = shared.tasks().register_task_with::<CreateCargoToml, _>(
            CREATE_CARGO_TOML,
            move |task, _project| {
                task.depends_on(script_tasks_clone.clone());
                let scripts: Vec<TaskProvider<_, _, _>> = script_tasks_clone
                    .into_iter()
                    .map(|t| t.provides(|t| AnonymousProvider::new(t.compiled_script())))
                    .collect();
                for script_provider in scripts {
                    let prop = AnonymousProvider::new(script_provider).flatten();
                    task.scripts.push_with(prop);
                }
                task.config_path.set_with(build_cargo_toml)?;
                Ok(())
            },
        )?;
        let script_tasks_clone = script_tasks.clone();
        let project_dir = build_logic_path.clone();
        let create_rust =
            shared
                .tasks()
                .register_task_with::<CreateLibRs, _>(CREATE_LIB_RS, |task, _| {
                    let scripts: Vec<_> = script_tasks
                        .into_iter()
                        .map(|t| t.provides(|t| t.output_file.clone()).flatten())
                        .collect();
                    task.project_dir.set(project_dir)?;
                    task.project_script_files
                        .set(FileSet::with_path_providers(scripts))?;
                    Ok(())
                })?;

        let dependencies = cargo_toml_task
            .provides(|t| t.dependencies.clone())
            .flatten();
        let build_cargo_file = cargo_toml_task
            .provides(|t| t.config_path.clone())
            .flatten();

        shared.apply_plugin::<RustBasePlugin>()?;

        // shared.with_mut(|p| -> ProjectResult {
        //     let mut rust_ext = p.extension_mut::<RustPluginExtension>().unwrap();
        //     rust_ext.toolchain.set(Toolchain::with_version(1, 64))?;
        //     Ok(())
        // })?;

        let cargo_file = build_logic_path.join("Cargo.toml");

        let modify_cargo = shared.tasks().register_task_with::<PatchCargoToml, _>(
            "patch-cargo-toml",
            |cargo_toml_task, _project| {
                cargo_toml_task.depends_on("install-default-toolchain");
                cargo_toml_task.dependencies.push_all_with(dependencies);
                cargo_toml_task
                    .build_cargo_file
                    .set_with(build_cargo_file)?;
                cargo_toml_task.cargo_file.set(cargo_file)?;
                Ok(())
            },
        )?;

        let compile = shared.tasks().register_task_with::<CompileProject, _>(
            COMPILE_BUILD_LOGIC_PROJECT,
            move |task, _project| {
                task.depends_on(cargo_toml_task);
                task.depends_on(modify_cargo);
                task.depends_on(script_tasks_clone);
                task.depends_on(create_rust);
                task.depends_on("install-default-toolchain");
                task.source(&build_logic_path);
                task.project_dir.set(build_logic_path)?;
                Ok(())
            },
        )?;

        let lib_provider = compile.provides(|t| t.lib.clone()).flatten();
        shared.with_mut(|p| {
            p.extension_mut::<BuildLogicExtension>()
                .ok_or(assemble_core::lazy_evaluation::Error::PropertyNotSet)
                .and_then(|e| e.built_library.set_with(lib_provider))
        })?;

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
        vec!["compile", project_name, "build", "script"].join("-")
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
            .and_then(|s| s.as_ref())
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
