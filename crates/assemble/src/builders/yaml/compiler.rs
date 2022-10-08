use crate::build_logic::plugin::compilation::{CompileLang, CompiledScript};
use crate::build_logic::plugin::script::languages::YamlLang;
use crate::build_logic::plugin::script::BuildScript;
use crate::builders::yaml::yaml_build_file::YamlBuild;

use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;

/// Compiles a yaml-based project
#[derive(Debug, Default)]
pub struct YamlCompiler;

impl CompileLang<YamlLang> for YamlCompiler {
    type Err = Error;

    fn compile(
        script: &BuildScript<YamlLang>,
        output_path: &Path,
    ) -> Result<CompiledScript, Self::Err> {
        let mut file = File::create(output_path)?;

        let yaml_build: YamlBuild = serde_yaml::from_slice(script.contents())?;

        trace!("yaml_build = {:#?}", yaml_build);

        let (dependency_includes, dependencies) = {
            if let Some(script) = yaml_build.script() {
                let mut plugins = vec![];
                let mut includes = vec![];
                for request in script.plugin_requests() {
                    let id = {
                        let id = request.id();
                        match id {
                            "std" => "assemble-std",
                            "rust" => "assemble-rust",
                            v => v,
                        }
                    };
                    if let Some(p_includes) = request.includes() {
                        includes.extend(
                            p_includes
                                .iter()
                                .map(|s| format!("{}::{s}", id.replace('-', "_"))),
                        )
                    }
                    if let Some(version) = request.version() {
                        plugins.push((id.to_string(), String::from(version)))
                    } else {
                        plugins.push((id.to_string(), String::from("*")))
                    }
                }
                (includes, plugins)
            } else {
                (vec![], vec![])
            }
        };
        let mut function = "".to_string();

        for (name, _) in &dependencies {
            let crate_name = name.replace('-', "_");
            function = format!(
                r"
{function}
    project.apply_plugin::<{crate_name}::Plugin>()?;
"
            )
        }

        for (id, request) in yaml_build.tasks() {
            let task_id = id.replace('-', "_");
            let task_ty = request.ty();
            function = format!(
                r"
{function}
    let mut {task_id} = project.task_container_mut().register_task::<{task_ty}>({id:?})?;
"
            );
            if let Some(depends_on) = request.depends_on() {
                let depends = depends_on
                    .iter()
                    .map(|s| format!("task.dependsOn({s:?});\n"))
                    .collect::<String>();
                function = format!(
                    r"
{function}
    {task_id}.configure_with(|task, project| {{
        {depends}
    }})?;
"
                );
            }

            if let Some(vars) = request.vars() {
                let vars = vars
                    .iter()
                    .map(|(key, value)| {
                        Ok(format!(
                            "       task.{key} = {};\n",
                            serde_yaml::to_string(value)?.trim()
                        ))
                    })
                    .collect::<Result<String, serde_yaml::Error>>()?;
                function = format!(
                    r"
{function}
    {task_id}.configure_with(|task, project| {{
{vars}
    }})?;
"
                );
            }

            if let Some(vars) = request.props() {
                let vars = vars
                    .iter()
                    .map(|(key, value)| {
                        Ok(format!(
                            "       task.{key}.set({})?;\n",
                            serde_yaml::to_string(value)?.trim()
                        ))
                    })
                    .collect::<Result<String, serde_yaml::Error>>()?;
                function = format!(
                    r"
{function}
    {task_id}.configure_with(|task, project| {{
{vars}
    }})?;
"
                );
            }
        }

        let includes = dependency_includes
            .into_iter()
            .map(|inc| format!("use {inc};\n"))
            .collect::<String>();
        let project_id = script.project();
        writeln!(
            file,
            r"// {project_id}
use assemble_core::prelude::*;
{includes}
pub fn configure(project: &mut Project) -> ProjectResult {{
    {function}

    Ok(())
}}",
        )?;

        let compiled = CompiledScript::new(output_path, dependencies);

        Ok(compiled)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    SerdeError(#[from] serde_yaml::Error),
    #[error(transparent)]
    IoError(#[from] io::Error),
}
