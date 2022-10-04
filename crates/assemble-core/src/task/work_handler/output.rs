use std::any::type_name;
use crate::file_collection::{FileCollection, FileSet};
use crate::prelude::ProjectError;
use crate::project::ProjectResult;
use crate::task::up_to_date::UpToDate;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::ops::Sub;
use std::path::PathBuf;
use std::time::SystemTime;
use crate::task::work_handler::serializer::from_str;

/// The output of a task.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Output {
    timestamp: SystemTime,
    files: HashSet<PathBuf>,
    serialized_data: Option<HashMap<String, String>>,
}

impl Output {
    /// Create a new output from a file collection
    pub fn new<F: FileCollection>(
        fc: F,
        serialized_data: impl Into<Option<HashMap<String, String>>>,
    ) -> Self {
        let files = fc.files();
        let timestamp = SystemTime::now();
        Self {
            timestamp,
            files,
            serialized_data: serialized_data.into(),
        }
    }

    /// Gets previously serialized data, if any set
    pub fn serialized_data(&self) -> Option<&HashMap<String, String>> {
        self.serialized_data.as_ref()
    }

    pub fn try_deserialize<T: DeserializeOwned>(path: &str) -> ProjectResult<T> {
        from_str(path)
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
