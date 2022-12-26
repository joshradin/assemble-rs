use assemble_core::file_collection::{FileCollection, FileSet};
use assemble_core::flow::output::SinglePathOutputTask;
use assemble_core::lazy_evaluation::ProviderExt;
use assemble_core::lazy_evaluation::{Prop, Provider};
use assemble_core::logging::{LoggingArgs, OutputType};
use assemble_core::prelude::TaskId;
use assemble_core::project::buildable::Buildable;
use assemble_core::project::error::ProjectResult;
use assemble_core::project::finder::TaskFinder;
use assemble_core::project::shared::SharedProject;
use assemble_core::task::initialize_task::InitializeTask;
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::{BuildResult, Executable, Project, Task};
use assemble_macros::{CreateTask, TaskIO};
use log::LevelFilter;
use more_collection_macros::set;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

static PROJECT: Lazy<SharedProject> = Lazy::new(init_project);
static TEMP_FILE: &str = "temp_file.txt";
static TEMP_DIR_DEST: &str = "dest";

#[derive(Debug, CreateTask, TaskIO)]
struct TestCopy {
    from: Prop<FileSet>,
    into: Prop<PathBuf>,
}

impl SinglePathOutputTask for TestCopy {
    fn get_path(task: &Executable<Self>) -> PathBuf {
        task.into.get()
    }
}

impl UpToDate for TestCopy {}

impl InitializeTask for TestCopy {
    fn initialize(task: &mut Executable<Self>, _project: &Project) -> ProjectResult {
        task.depends_on(task.from.clone());
        Ok(())
    }
}

impl Task for TestCopy {
    fn task_action(_task: &mut Executable<Self>, _project: &Project) -> BuildResult {
        todo!()
    }
}

fn base_file(path: impl AsRef<Path>) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from("C:\\").join(path)
    } else {
        PathBuf::from("/").join(path)
    }
}

fn init_project() -> SharedProject {
    let mut project = Project::temp("dependencies");

    let mut copy_file_handle = project.register_task::<TestCopy>("copyFile").unwrap();
    let mut copy_file_handle2 = project.register_task::<TestCopy>("copyFile2").unwrap();

    let mut configurations = project.configurations_mut();
    let mut config1 = configurations.create("config1").clone();
    config1.add_dependency(PathBuf::from(TEMP_FILE));

    let config2 = configurations
        .create_with("config2", |config| {
            config.add_dependency(copy_file_handle.clone())
        })
        .clone();

    drop(configurations);

    copy_file_handle
        .configure_with(|c, _p| {
            c.from.set_with(config1)?;
            c.into.set(TEMP_DIR_DEST)?;
            Ok(())
        })
        .unwrap();

    copy_file_handle2
        .configure_with(|c, _p| {
            c.from.set_with(config2)?;
            Ok(())
        })
        .unwrap();

    project
}
#[test]
fn tasks_transitive_task_dependencies_through_configurations() {
    LoggingArgs::init_root_logger_with(LevelFilter::Trace, OutputType::Basic);
    let project = &*PROJECT;
    let mut task = project.find_task("copyFile2").unwrap().clone();
    let dependencies = project
        .with(|p| -> Result<_, _> {
            println!("resolving task...");
            let built_by = task.resolve(p)?.built_by();
            println!("built_by = {:#?}", built_by);
            built_by.get_dependencies(p)
        })
        .unwrap();
    println!("dependencies: {:?}", dependencies);
    let finder = TaskFinder::new(project);
    assert_eq!(
        dependencies,
        set!(finder
            .find("copyFile")
            .ok()
            .flatten()
            .unwrap()
            .first()
            .cloned()
            .unwrap())
    )
}

#[test]
fn resolve_file_only_configuration() {
    let project = &*PROJECT;
    let config1 = project
        .configurations()
        .get("config1")
        .unwrap()
        .resolved()
        .unwrap();
    assert_eq!(config1.files(), HashSet::from_iter([base_file(TEMP_FILE)]));
}

#[test]
fn configuration_with_task_dependencies_resolves() {
    let project = &*PROJECT;
    let config2 = project.configurations().get("config2").cloned().unwrap();
    assert_eq!(
        config2.resolved().unwrap().files(),
        HashSet::from_iter([PathBuf::from(TEMP_DIR_DEST)])
    );
    let dependencies = project.with(|p| config2.get_dependencies(p)).unwrap();
    println!("dependencies: {:?}", dependencies);
    let finder = TaskFinder::new(project);
    assert_eq!(
        dependencies,
        set!(finder
            .find("copyFile")
            .ok()
            .flatten()
            .unwrap()
            .first()
            .cloned()
            .unwrap())
    )
}
