use crate::build_logic::plugin::compilation::CompiledScript;
use assemble_core::__export::Provider;
use assemble_core::lazy_evaluation::{Prop, VecProp};
use assemble_core::task::initialize_task::InitializeTask;
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::{BuildResult, Executable, Project, Task};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use toml_edit::{value, Document};

/// Creates a `Cargo.toml` file
#[derive(Debug, CreateTask, TaskIO)]
pub struct CreateCargoToml {
    #[input]
    pub scripts: VecProp<CompiledScript>,
    #[output(file)]
    pub config_path: Prop<PathBuf>,
    #[output]
    pub dependencies: VecProp<String>,
}

impl UpToDate for CreateCargoToml {}

impl InitializeTask for CreateCargoToml {}

impl Task for CreateCargoToml {
    fn task_action(task: &mut Executable<Self>, project: &Project) -> BuildResult {
        let mut dependencies = HashMap::new();
        for script in &task.scripts.fallible_get()? {
            debug!("script = {:?}", script);
            dependencies.extend(
                script
                    .dependencies()
                    .iter()
                    .map(|(dep, ver)| (dep.clone(), ver.clone())),
            )
        }
        info!(
            "dependencies for {} build-logic: {:#?}",
            project, dependencies
        );

        let toml = r#"
[package]
version = "0.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]
path = "lib.rs"

[dependencies]
        "#;

        let mut doc = toml.parse::<Document>().expect("invalid Cargo.toml");

        doc["package"]["name"] = value(
            project
                .id()
                .to_string()
                .replace(':', "_")
                .trim_matches(|s| s == '_'),
        );

        let core = assemble_core::version::version();

        doc["dependencies"][core.name()] = value(format!("<={}", core.version()));
        task.dependencies.push(core.name());
        for (dependency, version) in dependencies {
            doc["dependencies"][&dependency] = value(version);
            task.dependencies.push(dependency);
        }

        info!("Cargo.toml = {}", doc);

        let string = doc.to_string();
        let path = task.config_path.fallible_get()?;
        fs::create_dir_all(path.parent().unwrap())?;
        let mut file = File::create(path)?;

        writeln!(file, "{}", string)?;

        Ok(())
    }
}
