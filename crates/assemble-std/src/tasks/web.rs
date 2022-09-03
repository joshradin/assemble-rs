//! Contains web-based tasks, like getting

use crate::assemble_core::properties::ProvidesExt;
use assemble_core::identifier::TaskId;
use assemble_core::project::{ProjectError, ProjectResult};
use assemble_core::properties::{Prop, Provides};
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::task::{CreateTask, InitializeTask};
use assemble_core::{BuildResult, Executable, Project, Task};
use std::path::PathBuf;
use url::Url;

/// Downloads a file
#[derive(Debug, Clone, CreateTask, TaskIO)]
pub struct DownloadFile {
    /// The url to download from
    #[input]
    pub url: Prop<Url>,
    /// The file name to download into
    #[output]
    pub fname: Prop<PathBuf>,
}

impl InitializeTask for DownloadFile {
    fn initialize(task: &mut Executable<Self>, project: &Project) -> ProjectResult {
        let build_dir = project.build_dir();

        let map = task
            .url
            .clone()
            .zip(build_dir, |url: Url, build_dir: PathBuf| {
                build_dir.join("downloads").join(
                    url.path_segments()
                        .and_then(|segs| segs.last())
                        .and_then(|name| if name.is_empty() { None } else { Some(name) })
                        .unwrap_or("tmp.bin"),
                )
            });
        task.fname.set_with(map)?;
        Ok(())
    }
}

impl UpToDate for DownloadFile {}

impl Task for DownloadFile {
    fn task_action(task: &mut Executable<Self>, _project: &Project) -> BuildResult {
        let url = task.url.get();
        println!("url = {}", url);
        println!("fname = {:?}", task.fname.try_get().unwrap_or_default());

        Ok(())
    }
}
//
// impl CreateTask for DownloadFile {
//     fn new(id: &TaskId, project: &Project) -> ProjectResult<Self> {
//         let url = id.prop("url").unwrap();
//         let mut fname: Prop<PathBuf> = id.prop("file_name").unwrap();
//
//         let build_dir = project.build_dir();
//
//         let map = url.zip(&build_dir, |url: Url, build_dir: PathBuf| {
//             println!("Calculating fname");
//             build_dir.join("downloads").join(
//                 url.path_segments()
//                     .and_then(|segs| segs.last())
//                     .and_then(|name| if name.is_empty() { None } else { Some(name) })
//                     .unwrap_or("tmp.bin"),
//             )
//         });
//         fname.set_with(map).unwrap();
//         Ok(Self { url, fname })
//     }
//
//
// }
