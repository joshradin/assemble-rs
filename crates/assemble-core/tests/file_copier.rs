use assemble_core::exception::BuildError;
use assemble_core::file::RegularFile;
use assemble_core::prelude::*;
use assemble_core::project::ProjectResult;
use assemble_core::properties::Prop;
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::task::work_handler::InputFile;
use assemble_core::task::{ExecutableTask, InitializeTask};
use assemble_core::{BuildResult, Executable, Project, Task};
use assemble_macros::{CreateTask, TaskIO};
use std::path::PathBuf;
use tempfile::tempdir;

#[derive(Debug, CreateTask, TaskIO)]
pub struct CopyFile {
    #[input(file)]
    from: Prop<PathBuf>,
    #[output]
    into: Prop<PathBuf>,
}

impl UpToDate for CopyFile {}

impl InitializeTask for CopyFile {
    fn initialize(task: &mut Executable<Self>, _project: &Project) -> ProjectResult {
        // let from = task.from.clone();
        // let into = task.into.clone();
        // task.work().input_file("from", from)?;
        // task.work().add_output_provider(into);
        Ok(())
    }
}

impl Task for CopyFile {
    fn task_action(task: &mut Executable<Self>, _project: &Project) -> BuildResult {
        let mut from = RegularFile::open(task.from.get()).map_err(|e| {
            BuildError::with_inner(format!("couldn't open from file {:?}", task.from.get()), e)
        })?;
        let mut into = RegularFile::create(&task.into.get())?;

        std::io::copy(&mut from, &mut into)?;

        Ok(())
    }
}

#[test]
fn copy_task_up_to_date() {
    let temp_dir = tempdir().expect("couldn't make temp directory");

    let mut content = "Hello, World";

    for run in 0..3 {
        let mut project = Project::in_dir_with_id(temp_dir.path(), "test").unwrap();

        std::fs::write(temp_dir.path().join("from_file"), content).unwrap();

        let mut handle = project.register_task::<CopyFile>("copyFile").unwrap();
        let temp_dir_path = temp_dir.path().to_path_buf();
        handle
            .configure_with(move |handle, p| {
                handle.from.set(temp_dir_path.join("from_file"))?;
                handle.into.set(temp_dir_path.join("into_file"))?;
                Ok(())
            })
            .unwrap();
        let result = project.with(|p| handle.execute(p));
        assert!(result.is_ok(), "{}", result.unwrap_err());

        let found_content = std::fs::read_to_string(temp_dir.path().join("from_file"))
            .expect("couldn't read from file");
        assert_eq!(found_content, content);

        match run {
            0 => {
                assert!(!handle.up_to_date());
                assert!(handle.did_work());
            }
            1 => {
                assert!(handle.up_to_date());
                assert!(!handle.did_work());
                content = "Goodbye, world";
            }
            2 => {
                assert!(!handle.up_to_date());
                assert!(handle.did_work());
            }
            _ => unreachable!(),
        }
    }
}
