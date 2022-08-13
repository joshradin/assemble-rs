use crate::file_collection::{FileCollection, FileSet};
use crate::task::up_to_date::UpToDate;
use std::collections::HashSet;
use std::ops::Sub;
use std::path::PathBuf;
use std::time::SystemTime;

/// The output of a task.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Output {
    timestamp: SystemTime,
    files: HashSet<PathBuf>,
}

impl Output {
    /// Create a new output from a file collection
    pub fn new<F: FileCollection>(fc: F) -> Self {
        let files = fc.files();
        let timestamp = SystemTime::now();
        Self { timestamp, files }
    }
}

impl UpToDate for Output {
    /// The output of a task it not up to date if any files have been removed or added, or if a file is newer
    /// than the time stamp
    fn up_to_date(&self) -> bool {
        (|| -> Result<(), ()> {
            let regenerated_files = FileSet::from_iter(&self.files).files();
            if regenerated_files == self.files {
                for file in regenerated_files {
                    let meta_data = file.metadata().map_err(|_| ())?;
                    if meta_data.modified().map_err(|_| ())? > self.timestamp {
                        return Err(());
                    }
                }
                Ok(())
            } else {
                Err(())
            }
        })()
        .is_ok()
    }
}
