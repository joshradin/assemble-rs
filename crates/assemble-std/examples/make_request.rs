use assemble_core::identifier::{ProjectId, TaskId};
use assemble_core::immutable::Immutable;
use assemble_core::prelude::Provides;
use assemble_core::prelude::ProvidesExt;
use assemble_core::project::ProjectError;
use assemble_core::properties::providers::FlatMap;
use assemble_core::task::task_container::TaskConfigurator;
use assemble_core::Project;
use assemble_std::tasks::web::DownloadFile;
use reqwest::Url;
use std::str::FromStr;
use assemble_core::task::executable::Executable;

fn main() {
    let project = create_project().unwrap();

    println!("project: {:?}", project);

    let id = TaskId::new("root:downloadRustSh").unwrap();
    let mut configured = project
        .task_container()
        .resolve_task(id.clone(), &project)
        .unwrap();

    let p = project.task_container().get_task_provider::<DownloadFile>(&id).unwrap();
    let url = p.flat_map(|p| p.url.clone()).get();

    println!("url from provider = {}", url);

    configured.execute(&project).unwrap();
}

fn create_project() -> Result<Project, ProjectError> {


    let mut project = Project::with_id("root")?;

    let mut provider = project.task::<DownloadFile>("downloadRustSh")?;
    provider.configure_with(|task, _, _| {
        task.url.set(
            Url::from_str("https://raw.githubusercontent.com/joshradin/joshradin/main/README.md")
                .unwrap(),
        )?;
        Ok(())
    });

    Ok(project)
}
