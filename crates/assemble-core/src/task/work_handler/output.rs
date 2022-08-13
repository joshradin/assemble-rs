use crate::file_collection::FileSet;
use std::time::SystemTime;

/// The output of a task
pub struct Output {
    timestamp: SystemTime,
    files: FileSet,
}
