use std::path::PathBuf;
use assemble_core::Project;
use assemble_core::project::ProjectError;

#[test]
fn inter_project_dependencies() -> Result<(), ProjectError>{

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


    Ok(())
}