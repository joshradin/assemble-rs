use assemble_core::project::ProjectError;
use assemble_core::Project;
use std::path::PathBuf;
use assemble_core::dependencies::project_dependency::{project_url, subproject_url};

#[test]
fn inter_project_dependencies() -> Result<(), ProjectError> {
    let project = Project::temp("sub-projects-test");
    project.with_mut(|p| -> Result<(), ProjectError> {
        p.subproject("child1", |sub| {
            println!("in dir: {:?}", sub.project_dir());
            Ok(())
        })?;
        p.subproject("child2", |sub| {
            println!("in dir: {:?}", sub.project_dir());
            Ok(())
        })?;
        println!("children: {:?}", p.subprojects());
        Ok(())
    })?;

    let child1 = project.get_subproject("child1").unwrap();
    println!("child1: {:#?}", child1);

    println!("child1 url = {}", project_url(&child1));
    println!("root url = {}", subproject_url(&child1, ":", None)?);
    println!("child2 (relative from child1) url = {}", subproject_url(&child1, "child2", None)?);


    Ok(())
}
