use assemble_core::__export::{InitializeTask, Provider};
use assemble_core::{BuildResult, Executable, Project, Task};
use assemble_core::task::up_to_date::UpToDate;
use std::collections::HashMap;
use toml_edit::{Document, value};
use std::fs::File;
use assemble_core::lazy_evaluation::{Prop, VecProp};
use std::path::PathBuf;
use crate::build_logic::plugin::compilation::CompiledScript;
use std::io::Write;

/// Creates a `Cargo.toml` file
#[derive(Debug, CreateTask, TaskIO)]
pub struct CreateCargoToml {
    pub scripts: VecProp<CompiledScript>,
    #[output]
    pub config_path: Prop<PathBuf>,
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
                    .into_iter()
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
        for (dependency, version) in dependencies {
            doc["dependencies"][dependency] = value(version);
        }

        info!("Cargo.toml = {}", doc);

        let string = doc.to_string();
        let mut file = File::create(task.config_path.fallible_get()?)?;

        writeln!(file, "{}", string)?;

        Ok(())
    }
}
