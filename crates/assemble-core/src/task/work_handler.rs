use crate::cryptography::{hash_file_sha256, Sha256};
use crate::exception::{BuildError, BuildException};
use crate::file_collection::{FileCollection, FileSet};
use crate::identifier::{Id, TaskId};
use crate::project::ProjectResult;
use crate::properties::{IntoProvider, Prop, Provides, ProvidesExt};
use crate::task::up_to_date::UpToDate;
use crate::task::work_handler::output::Output;
use crate::Project;
use input::Input;
use log::{info, trace};
use once_cell::sync::{Lazy, OnceCell};
use ron::ser::PrettyConfig;
use serde::de::DeserializeOwned;
use serde::ser::Error as _;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::{create_dir_all, File};
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use time::{OffsetDateTime, PrimitiveDateTime};

pub mod input;
pub mod output;

pub struct WorkHandler {
    task_id: TaskId,
    cache_location: PathBuf,
    inputs: Vec<Prop<String>>,
    outputs: Option<FileSet>,
    final_input: OnceCell<Input>,
    final_output: OnceCell<Option<Output>>,
    execution_history: OnceCell<TaskExecutionHistory>,
    up_to_date_status: OnceCell<bool>,
    did_work: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct TaskExecutionHistory {
    input: Input,
    output: Output,
}

impl WorkHandler {
    pub fn new(id: &TaskId, cache_loc: PathBuf) -> Self {
        Self {
            task_id: id.clone(),
            cache_location: cache_loc,
            inputs: vec![],
            outputs: None,
            final_input: OnceCell::new(),
            final_output: OnceCell::new(),
            execution_history: OnceCell::new(),
            up_to_date_status: OnceCell::new(),
            did_work: true,
        }
    }

    pub fn has_inputs_and_outputs(&self) -> bool {
        !self.inputs.is_empty() && self.outputs.is_some()
    }

    /// Removes execution history, if it exists.
    pub fn remove_execution_history(&self) -> io::Result<()> {
        let path = self.task_id.as_path();
        let file_location = self.cache_location.join(path);
        if file_location.exists() {
            std::fs::remove_file(file_location)?;
        }
        Ok(())
    }

    /// Store execution data. Will only perform a store if there's both an input and an output
    pub fn store_execution_history(&self) -> io::Result<()> {
        let input = self.get_input().clone();
        if !input.any_inputs() {
            return Ok(());
        }
        let output = if let Some(output) = self.get_output().clone() {
            output.clone()
        } else {
            return Ok(());
        };
        let history = TaskExecutionHistory { input, output };
        let path = self.task_id.as_path();
        let file_location = self.cache_location.join(path);
        if let Some(parent) = file_location.parent() {
            create_dir_all(parent)?;
        }

        let mut file = File::options()
            .write(true)
            .truncate(true)
            .create(true)
            .open(file_location)?;

        ron::ser::to_writer_pretty(&mut file, &history, PrettyConfig::new()).unwrap();
        Ok(())
    }

    pub fn cache_input(&self, input: Input) -> io::Result<()> {
        let path = self.task_id.as_path();
        let file_location = self.cache_location.join(path);
        if let Some(parent) = file_location.parent() {
            create_dir_all(parent)?;
        }

        let mut file = File::options()
            .write(true)
            .truncate(true)
            .create(true)
            .open(file_location)?;

        ron::ser::to_writer_pretty(&mut file, &input, PrettyConfig::new()).unwrap();
        Ok(())
    }

    fn try_get_execution_history(&self) -> Option<&TaskExecutionHistory> {
        self.execution_history
            .get_or_try_init(|| -> Result<TaskExecutionHistory, Box<dyn Error>> {
                let path = self.task_id.as_path();
                let file_location = self.cache_location.join(path);
                if file_location.exists() {
                    let mut read = File::open(&file_location)?;
                    let mut buffer = String::new();
                    read.read_to_string(&mut buffer)
                        .expect(&format!("Could not read to end of {:?}", file_location));
                    Ok(ron::from_str(&buffer)?)
                } else {
                    Err(Box::new(BuildError::new("no file found for cache")))
                }
            })
            .ok()
    }

    pub fn try_get_prev_input(&self) -> Option<&Input> {
        self.try_get_execution_history().map(|h| &h.input)
    }

    pub fn add_input<T: Serialize + Send + Sync + Clone + 'static, P: IntoProvider<T>>(
        &mut self,
        id: &str,
        value: P,
    ) -> ProjectResult
    where
        <P as IntoProvider<T>>::Provider: 'static,
    {
        let mut prop: Prop<String> = self.task_id.prop(id)?;
        let value_provider = value.into_provider();
        prop.set_with(value_provider.flat_map(Self::serialize_data))?;
        self.inputs.push(prop);
        Ok(())
    }

    pub fn add_input_file<Pa: AsRef<Path> + 'static, P: IntoProvider<Pa>>(
        &mut self,
        id: &str,
        value: P,
    ) -> ProjectResult
    where
        Pa: Send + Sync + Clone,
        <P as IntoProvider<Pa>>::Provider: 'static + Clone,
    {
        let mut prop: Prop<String> = self.task_id.prop(id)?;
        let provider = value.into_provider();
        let path_provider = provider.flat_map(|p| Self::serialize_data(InputFile::new(p.as_ref())));
        prop.set_with(path_provider)?;
        self.inputs.push(prop);
        Ok(())
    }

    pub fn add_input_prop<T: Serialize + Send + Sync + Clone + 'static>(
        &mut self,
        prop: &Prop<T>,
    ) -> ProjectResult {
        let mut string_prop: Prop<String> = Prop::new(prop.id().clone());
        string_prop.set_with(prop.clone().flat_map(Self::serialize_data))?;
        self.inputs.push(string_prop);
        Ok(())
    }

    pub fn get_input(&self) -> &Input {
        self.final_input
            .get_or_init(|| Input::new(&self.task_id, &self.inputs))
    }

    /// Add some output file collection. Can add outputs until [`get_output`](WorkHandler::get_output) is called.
    pub fn add_output<F: FileCollection>(&mut self, fc: F) {
        *self.outputs.get_or_insert(FileSet::new()) += FileSet::from_iter(fc.files());
    }

    /// Add some output file collection. Can add outputs until [`get_output`](WorkHandler::get_output) is called.
    pub fn add_output_provider<P, F>(&mut self, fc_provider: P)
    where
        P: Provides<F> + 'static,
        F: FileCollection + Send + Sync + Clone + 'static,
    {
        *self.outputs.get_or_insert(FileSet::new()) += FileSet::with_provider(fc_provider);
    }

    /// Get the output of this file collection
    pub fn get_output(&self) -> Option<&Output> {
        self.final_output
            .get_or_init(|| self.outputs.as_ref().map(|o| Output::new(o.clone())))
            .as_ref()
    }

    pub fn prev_work(&self) -> Option<(&Input, &Output)> {
        self.try_get_prev_input().zip(self.try_get_prev_output())
    }

    /// Try to get the output of the previous run
    pub fn try_get_prev_output(&self) -> Option<&Output> {
        self.try_get_execution_history().map(|h| &h.output)
    }

    pub fn did_work(&self) -> bool {
        self.did_work
    }

    pub fn set_did_work(&mut self, did_work: bool) {
        self.did_work = did_work;
    }

    pub fn set_up_to_date(&mut self, up_to_date_status: bool) {
        self.up_to_date_status
            .set(up_to_date_status)
            .expect("up to date status already set")
    }

    pub fn up_to_date(&self) -> &bool {
        self.up_to_date_status
            .get()
            .expect("up to date status not set")
    }

    fn serialize_data<T: Serialize>(val: T) -> impl Provides<String> {
        ron::to_string(&val)
    }
}

pub struct InputFile(PathBuf);

impl InputFile {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        Self(path)
    }

    /// Direct implementaiton of serialize
    pub fn serialize<P : AsRef<Path>, S : Serializer>(path: P, serializer: S) -> Result<S::Ok, S::Error> {
        Self::new(path).serialize(serializer)
    }
}

#[derive(Serialize)]
struct InputFileData {
    path: PathBuf,
    data: Sha256,
}

impl Serialize for InputFile {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.0.exists() {
            InputFileData {
                path: self.0.clone(),
                data: hash_file_sha256(&self.0).map_err(|e| S::Error::custom(e))?,
            }
            .serialize(serializer)
        } else {
            ().serialize(serializer)
        }
    }
}

#[derive(Default)]
pub enum ChangeStatus {
    /// Value was deleted.
    Deleted,
    /// Value was modified
    Modified,
    #[default]
    Added,
    Same,
}

/// Normalizes some system time to UTC time
pub fn normalize_system_time(system_time: SystemTime) -> OffsetDateTime {
    let duration = system_time
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Couldn't determine duration since UNIX EPOCH");
    let start = OffsetDateTime::UNIX_EPOCH;
    start + duration
}
