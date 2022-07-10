//! Contains web-based tasks, like getting

use crate::assemble_core::properties::ProvidesExt;
use assemble_core::__export::TaskProperties;
use assemble_core::file_collection::Component::Path;
use assemble_core::identifier::TaskId;
use assemble_core::project::ProjectError;
use assemble_core::properties::{Provides, Prop};
use assemble_core::task::{CreateTask, DynamicTaskAction, CreateDefaultTask};
use assemble_core::{BuildResult, DefaultTask, Project, Task};
use reqwest::Url;
use std::path::PathBuf;

/// Downloads a file
#[derive(Debug, Task, Clone)]
pub struct DownloadFile {
    /// The url to download from
    pub url: Prop<Url>,
    /// The file name to download into
    pub fname: Prop<PathBuf>,
}

impl DynamicTaskAction for DownloadFile {
    fn exec(&mut self, project: &Project) -> BuildResult {
        let url = self.url.get();
        println!("url = {}", url);
        println!("fname = {:?}", self.fname.get());

        Ok(())
    }
}

impl CreateDefaultTask for DownloadFile {
    fn new_default_task() -> Self {
        Self {
            url: Prop::default(),
            fname: Prop::default()
        }
    }
}

impl CreateTask for DownloadFile {
    fn create_task(id: TaskId, project: &Project) -> Self {
        let url = id.prop("url").unwrap();
        let mut fname: Prop<PathBuf> = id.prop("file_name").unwrap();

        let build_dir = project.build_dir();

        let map = url.zip(&build_dir, |url: Url, build_dir: PathBuf| {
            println!("Calculating fname");
                build_dir.join("downloads").join(
                url.path_segments()
                    .and_then(|segs| segs.last())
                    .and_then(|name| if name.is_empty() { None } else { Some(name) })
                    .unwrap_or("tmp.bin"))

        });
        fname.set_with(map).unwrap();
        Self { url, fname }
    }
}
