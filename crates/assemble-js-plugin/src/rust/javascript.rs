//! Gets the typescript definitions

use include_dir::{Dir, File};
use log::{info, log, Level};
use rquickjs::bind;
use std::path::{Path, PathBuf};
use thiserror::Error;
use assemble_core::payload_from;

static TYPESCRIPT: Dir<'_> = include_dir::include_dir!("$CARGO_MANIFEST_DIR/src/ts");
static TRANSPILED_JAVASCRIPT: Dir<'_> = include_dir::include_dir!("$OUT_DIR/js");

pub mod listeners;
pub mod logger;
pub mod project;
pub mod task;
pub use logger::Logging;

/// Gets a file from the transpiled java script
pub fn file<'a, P: AsRef<Path>>(path: P) -> Option<&'a File<'static>> {
    TRANSPILED_JAVASCRIPT.get_file(path)
}

/// Gets a file from the transpiled java script
pub fn file_contents<'a, P: AsRef<Path>>(path: P) -> Result<&'a str, FileError> {
    let path = path.as_ref();
    let file = TRANSPILED_JAVASCRIPT
        .get_file(path)
        .ok_or(FileError::FileMissing(path.to_path_buf()))?;

    file.contents_utf8()
        .ok_or(FileError::FileNotUf8(path.to_path_buf()))
}

#[derive(Debug, Error)]
pub enum FileError {
    #[error("No transpiled file with path: {0:?}")]
    FileMissing(PathBuf),
    #[error("Transpiled file with path {0:?} not uft-8 encoded")]
    FileNotUf8(PathBuf),
}

const REQUIRES_STATE: &str = "__imported_requires";

#[bind(object, public)]
#[quickjs(bare)]
mod bindings {
    use crate::javascript::{file_contents, REQUIRES_STATE};
    use rquickjs::Ctx;
    use std::collections::HashSet;
    use std::path::{Path, PathBuf, MAIN_SEPARATOR};

    pub fn require(ctx: Ctx, path: String) {
        let mut path = PathBuf::from(path);
        path.set_extension("js");

        if !ctx.globals().contains_key(REQUIRES_STATE).unwrap() {
            ctx.globals()
                .set(REQUIRES_STATE, HashSet::<String>::new())
                .expect("could not add state");
        }

        let mut imported = ctx
            .globals()
            .get::<_, HashSet<String>>(REQUIRES_STATE)
            .unwrap();

        log::trace!("imported = {:?}", imported);
        let import = format!("{:?}", path);
        if imported.contains(&import) {
            return;
        } else {
            imported.insert(import);
            ctx.globals().set(REQUIRES_STATE, imported).unwrap();
        }

        let contents =
            file_contents(&path).unwrap_or_else(|_| panic!("could not get file {:?}", path));

        ctx.eval::<(), _>(contents).expect("could not evaluate");
    }

    pub fn path_separator() -> String {
        format!("{}", MAIN_SEPARATOR)
    }
}

#[cfg(test)]
mod tests {
    use crate::javascript::TRANSPILED_JAVASCRIPT;

    #[test]
    fn files_linked() {
        assert!(
            TRANSPILED_JAVASCRIPT.entries().len() > 0,
            "no files detected"
        );
    }

    #[test]
    fn get_project_js() {
        let project_js = TRANSPILED_JAVASCRIPT
            .get_file("project.js")
            .expect("project.js file should exist");
        let string = project_js.contents_utf8().unwrap();
        println!("{}", string);
        assert!(
            string.contains("class Project"),
            "should contain project definition"
        );
    }

    #[test]
    fn get_task_js() {
        let project_js = TRANSPILED_JAVASCRIPT
            .get_file("tasks/task.js")
            .expect("tasks/task.js file should exist");
        let string = project_js.contents_utf8().unwrap();
        println!("{}", string);
        assert!(
            string.contains("class DefaultTask"),
            "should contain project definition"
        );
    }
}
