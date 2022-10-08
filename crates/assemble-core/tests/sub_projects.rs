use assemble_core::dependencies::project_dependency::CreateProjectDependencies;
use assemble_core::error::PayloadError;
use assemble_core::flow::output::SinglePathOutputTask;

use assemble_core::identifier::TaskId;
use assemble_core::lazy_evaluation::{Prop, Provider};
use assemble_core::project::buildable::Buildable;
use assemble_core::project::error::ProjectError;
use assemble_core::task::initialize_task::InitializeTask;
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::{BuildResult, Executable, Project, Task};
use assemble_macros::{CreateTask, TaskIO};

use std::collections::HashSet;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, CreateTask, TaskIO)]
struct TestArtifactTask {
    output: Prop<PathBuf>,
}

impl UpToDate for TestArtifactTask {}
impl InitializeTask for TestArtifactTask {}

impl Task for TestArtifactTask {
    fn task_action(_task: &mut Executable<Self>, _project: &Project) -> BuildResult {
        todo!()
    }
}

impl SinglePathOutputTask for TestArtifactTask {
    fn get_path(task: &Executable<Self>) -> PathBuf {
        task.output.get()
    }
}

#[test]
fn inter_project_dependencies() -> Result<(), PayloadError<ProjectError>> {
    let project = Project::temp("sub-projects-test");
    project.with_mut(|p| -> Result<(), PayloadError<ProjectError>> {
        p.subproject("child1", |_sub| Ok(()))?;
        p.subproject("child2", |_sub| Ok(()))?;
        println!("children: {:?}", p.subprojects());
        Ok(())
    })?;

    let child1 = project.get_subproject("child1")?;
    println!("child1: {:#?}", child1);
    child1.with_mut(|p| -> Result<_, PayloadError<ProjectError>> {
        let mut task = p
            .task_container_mut()
            .register_task::<TestArtifactTask>("createFile")?;
        let file = p.file("testFile.txt")?.path().to_path_buf();
        let file_clone = file.clone();
        task.configure_with(|t, _p| {
            t.output.set(file_clone)?;
            Ok(())
        })?;
        p.variants_mut()
            .add_with("output", file, |c| c.built_by(task));
        Ok(())
    })?;

    let child2 = project.get_subproject("child2")?;
    println!("child2: {:#?}", child2);
    let config2 = child2.with_mut(|p| {
        let dep = p.project("::child1");
        p.configurations_mut()
            .create_with("input", |c| c.add_dependency(dep))
            .clone()
    });

    println!("config = {config2:#?}");
    let deps = child2.with(|p| config2.get_dependencies(p))?;
    println!("built by = {:#?}", deps);
    assert_eq!(
        deps,
        HashSet::from_iter([TaskId::from_str(":sub-projects-test:child1:createFile")?])
    );

    Ok(())
}
