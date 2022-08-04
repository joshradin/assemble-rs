use assemble_core::__export::{InitializeTask, TaskId};
use assemble_core::file_collection::{FileCollection, FileSet};
use assemble_core::flow::output::SinglePathOutputTask;
use assemble_core::project::{ProjectResult, SharedProject};
use assemble_core::properties::{Prop, Provides};
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::{BuildResult, Executable, Project, Task};
use assemble_core::properties::ProvidesExt;
use assemble_macros::CreateTask;
use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use more_collection_macros::set;
use assemble_core::project::buildable::Buildable;
use assemble_core::task::task_container::FindTask;

static PROJECT: Lazy<SharedProject> = Lazy::new(init_project);
static TEMP_FILE: &str = "C:/temp_file.txt";
static TEMP_DIR_DEST: &str = "C:/dest";

#[derive(Debug, CreateTask)]
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

fn init_project() -> SharedProject {
    let mut project = Project::temp("dependencies");

    let mut copy_file_handle = project.register_task::<TestCopy>("copyFile").unwrap();
    let mut copy_file_handle2 = project.register_task::<TestCopy>("copyFile2").unwrap();

    let mut configurations = project.configurations_mut();
    let mut config1 = configurations.create("config1").clone();
    config1.add_dependency(PathBuf::from(TEMP_FILE));

    let config2 = configurations.create_with("config2", |config| config.add_dependency(copy_file_handle.clone())).clone();

    drop(configurations);

    copy_file_handle.configure_with(|c, p| {
        c.from.set_with(config1)?;
        c.into.set(TEMP_DIR_DEST)?;
        Ok(())
    }).unwrap();

    copy_file_handle2.configure_with(|c, p| {
        c.from.set_with(config2)?;
        Ok(())
    }).unwrap();

    project
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
    assert_eq!(
        config1.files(),
        HashSet::from_iter([PathBuf::from(TEMP_FILE)])
    );
}

#[test]
fn configuration_with_task_dependencies_resolves() {
    let project = &*PROJECT;
    let config2 = project
        .configurations()
        .get("config2")
        .unwrap()
        .resolved()
        .unwrap();
    assert_eq!(
        config2.files(),
        HashSet::from_iter([PathBuf::from(TEMP_DIR_DEST)])
    );
    let dependencies = project.with(|p| config2.get_dependencies(p)).unwrap();
    println!("dependencies: {:?}", dependencies);
    assert_eq!(dependencies, set!(
        project.find_eligible_tasks("copyFile").ok().flatten().unwrap().first().cloned().unwrap()
    ))
}

#[test]
fn tasks_transitive_task_dependencies_through_configurations() {
    let project = &*PROJECT;
    let mut task = project.task_container().get_task("copyFile2").unwrap();
    let dependencies = project.with(|p| -> Result<_, _> {
        println!("resolving task");
        let built_by = task.resolve(p)?.built_by();
        built_by.get_dependencies(p)
    }).unwrap();
    println!("dependencies: {:?}", dependencies);
    assert_eq!(dependencies, set!(
        project.find_eligible_tasks("copyFile").ok().flatten().unwrap().first().cloned().unwrap()
    ))
}

