//! Tasks to patch cargo toml files

use assemble_core::__export::{CreateTask, InitializeTask, ProjectResult, TaskIO, TaskId};
use assemble_core::exception::{BuildError, BuildException};
use assemble_core::lazy_evaluation::anonymous::AnonymousProvider;
use assemble_core::lazy_evaluation::{Prop, Provider, VecProp};
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::task::BuildableTask;
use assemble_core::{cargo, BuildResult, Executable, Project, Task};
use assemble_std::specs::exec_spec::Output;
use assemble_std::ProjectExec;
use std::collections::HashMap;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use toml_edit::Value;
use toml_edit::{value, Document, InlineTable, Item, Table};

/// Patch cargo toml files
#[derive(Debug, CreateTask, TaskIO)]
pub struct PatchCargoToml {
    /// The input dependencies to add patches for
    #[input]
    pub dependencies: VecProp<String>,
    #[input(file)]
    pub build_cargo_file: Prop<PathBuf>,
    #[output(file)]
    pub cargo_file: Prop<PathBuf>,
    crates: HashMap<String, PathBuf>,
}

impl UpToDate for PatchCargoToml {}

impl InitializeTask for PatchCargoToml {
    fn initialize(task: &mut Executable<Self>, project: &Project) -> ProjectResult {
        Ok(())
    }
}

impl Task for PatchCargoToml {
    fn task_action(task: &mut Executable<Self>, project: &Project) -> BuildResult {



        if cargo::get_cargo_env().is_none() {
            fs::create_dir_all(task.cargo_file.fallible_get()?.parent().unwrap())?;
            let target_file = task.cargo_file.fallible_get()?;
            fs::copy(
                task.build_cargo_file.fallible_get()?,
                target_file
            )?;
            return Err(BuildException::StopTask.into());
        }

        if let Some(cargo) = cargo::get_cargo_env() {
            info!("got cargo env");
            let crate_name = cargo.package_name();
            let manifest_dir = cargo.manifest_directory();
            println!("directly working with {} in {:?}", crate_name, manifest_dir);

            let parent = manifest_dir.parent().unwrap();
            for dir in fs::read_dir(parent)? {
                let file = dir?;
                if file.metadata()?.is_dir() {
                    let cargo_file_path = file.path().join("Cargo.toml");
                    if !cargo_file_path.exists() {
                        continue;
                    }
                    info!("found package in dir: {:?}", file);
                    let mut cargo_file = File::open(cargo_file_path)?;
                    let mut doc = String::new();
                    cargo_file.read_to_string(&mut doc)?;
                    let doc = doc.parse::<Document>()?;
                    let name = doc["package"]["name"].as_str().unwrap();
                    task.crates.insert(name.to_string(), file.path());
                }
            }
        }

        info!("available patches: {:#?}", task.crates);

        let mut patches = HashMap::new();

        for dependency in task.dependencies.fallible_get()? {
            info!("dependency = {:?}", dependency);
            if task.crates.contains_key(&dependency) {
                info!("found known dependency: {}", dependency);
                let path = task.crates[&dependency].clone();
                patches.insert(dependency, path);
            }
        }

        if !patches.is_empty() {
            let mut doc = {
                let mut file = task.build_cargo_file.read()?;
                let mut string = String::new();
                file.read_to_string(&mut string)?;
                let doc = string.parse::<Document>()?;
                doc
            };

            doc.insert("patch", Item::Table(Table::new()));
            doc["patch"]
                .as_table_mut()
                .unwrap()
                .insert("crates-io", Item::Table(Table::new()));
            let mut patches_table = doc["patch"]["crates-io"]
                .as_table_mut()
                .ok_or(BuildError::new("not a known table"))?;

            for (dep, path) in patches {
                let mut inline = InlineTable::new();
                inline.insert("path", Value::from(path.to_string_lossy().as_ref()));
                patches_table[&dep] = value(inline);
            }

            {
                let string = doc.to_string();
                let mut file = task.cargo_file.open(OpenOptions::new().write(true))?;
                writeln!(file, "{}", string)?;
            }

            let parent = task
                .cargo_file
                .fallible_get()?
                .parent()
                .ok_or(BuildError::new("Cargo.toml file has no parent directory"))?
                .to_path_buf();

            project
                .exec_with(|e| {
                    e.exec("cargo")
                        .arg("update")
                        .working_dir(parent)
                        .stderr(Output::Null);
                })?
                .expect_success()?;
        }

        return Ok(());
    }
}
