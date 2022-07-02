use std::str::FromStr;
use reqwest::Url;
use assemble_core::identifier::{ProjectId, TaskId};
use assemble_core::{Executable, Project};
use assemble_core::project::ProjectError;
use assemble_core::task::task_container::TaskConfigurator;
use assemble_std::tasks::web::DownloadFile;

fn main() {

    let project = create_project().unwrap();


    println!("project: {:?}", project);

    let mut configured = project.task_container().resolve_task(
        TaskId::new("root:downloadRustSh").unwrap(),
        &project
    ).unwrap();

    configured.execute(&project).unwrap();

}

fn create_project() -> Result<Project, ProjectError> {

    let mut project = Project::with_id("root")?;
    project.task::<DownloadFile>("downloadRustSh")?.configure_with(|task, _, _| {
        task.url.set(Url::from_str("https://raw.githubusercontent.com/joshradin/joshradin/main/README.md").unwrap())?;
        Ok(())
    });

    project.set_build_dir("target");

    Ok(project)
}