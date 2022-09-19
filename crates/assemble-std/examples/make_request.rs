use assemble_core::error::PayloadError;
use assemble_core::identifier::{ProjectId, TaskId};
use assemble_core::immutable::Immutable;
use assemble_core::lazy_evaluation::providers::FlatMap;
use assemble_core::prelude::Provider;
use assemble_core::prelude::ProviderExt;
use assemble_core::project::error::ProjectError;
use assemble_core::project::SharedProject;
use assemble_core::task::task_container::FindTask;
use assemble_core::Project;
use assemble_std::tasks::web::DownloadFile;
use reqwest::Url;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::str::FromStr;

fn main() {
    let project = create_project().unwrap();

    println!("project: {:?}", project);

    let id = TaskId::new("root:downloadRustSh").unwrap();
    let mut configured = project.get_task(&id).unwrap();

    let p = project
        .task_container()
        .get_task(&id)
        .unwrap()
        .as_type::<DownloadFile>()
        .unwrap();
    let url = p.provides(|url| url.url.clone()).get();

    println!("url from provider = {}", url.get());

    let mut resolved = configured.resolve_shared(&project).unwrap();
    project.with(|p| resolved.execute(p)).unwrap();
}

fn create_project() -> Result<SharedProject, PayloadError<ProjectError>> {
    let mut project = Project::with_id("root")?;

    let mut provider = project.register_task::<DownloadFile>("downloadRustSh")?;
    provider
        .configure_with(|task, _| {
            task.url.set(
                Url::from_str(
                    "https://raw.githubusercontent.com/joshradin/joshradin/main/README.md",
                )
                .unwrap(),
            )?;
            task.do_first(|s, _| {
                println!("self = {:#?}", s);
                Ok(())
            })
        })
        .unwrap();

    Ok(project)
}
