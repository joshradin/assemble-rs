use crate::__export::from_str;
use crate::cryptography::{hash_file_sha256, Sha256};
use crate::exception::{BuildError, BuildException};
use crate::file_collection::{FileCollection, FileSet};
use crate::identifier::{Id, TaskId};
use crate::lazy_evaluation::anonymous::AnonymousProvider;
use crate::lazy_evaluation::{IntoProvider, Prop, Provider, ProviderExt, VecProp};
use crate::project::buildable::{BuiltByContainer, IntoBuildable};
use crate::project::error::ProjectResult;
use crate::task::up_to_date::UpToDate;
use crate::task::work_handler::output::Output;
use crate::task::work_handler::serializer::Serializable;
use crate::{provider, Project};
use input::Input;
use log::{info, trace};
use once_cell::sync::{Lazy, OnceCell};
use serde::de::DeserializeOwned;
use serde::ser::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::{create_dir_all, File};
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use time::{OffsetDateTime, PrimitiveDateTime};

pub mod input;
pub mod output;
pub mod serializer;

pub struct WorkHandler {
    task_id: TaskId,
    cache_location: PathBuf,
    inputs: VecProp<Serializable>,
    outputs: Option<FileSet>,
    serialized_output: HashMap<String, AnonymousProvider<Serializable>>,
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
            inputs: VecProp::new(id.join("inputs").unwrap()),
            outputs: None,
            serialized_output: Default::default(),
            final_input: OnceCell::new(),
            final_output: OnceCell::new(),
            execution_history: OnceCell::new(),
            up_to_date_status: OnceCell::new(),
            did_work: true,
        }
    }

    pub fn has_inputs_and_outputs(&self) -> bool {
        !self.inputs.get().is_empty() && self.outputs.is_some()
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
    pub fn store_execution_history(&self) -> ProjectResult<()> {
        let input = self.get_input()?.clone();
        if !input.any_inputs() {
            return Ok(());
        }
        let output = if let Some(output) = self.get_output()?.clone() {
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

        serializer::to_writer(&mut file, &history)?;
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

        serializer::to_writer(&mut file, &input).unwrap();
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
                    Ok(from_str(&buffer)?)
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
        let mut prop: Prop<Serializable> = self.task_id.prop(id)?;
        let value_provider = value.into_provider();
        prop.set_with(value_provider.flat_map(|v| Serializable::new(v)))?;
        self.inputs.push_with(prop);
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
        let mut prop: Prop<Serializable> = self.task_id.prop(id)?;
        let provider = value.into_provider();
        let path_provider = provider.flat_map(|p| Serializable::new(InputFile::new(p.as_ref())));
        prop.set_with(path_provider)?;
        self.inputs.push_with(prop);
        Ok(())
    }

    pub fn add_input_files<Pa, P: IntoProvider<Pa>>(&mut self, id: &str, value: P) -> ProjectResult
    where
        Pa: FileCollection,
        Pa: Send + Sync + Clone + 'static,
        <P as IntoProvider<Pa>>::Provider: 'static + Clone,
    {
        let mut prop: Prop<Serializable> = self.task_id.prop(id)?;
        let provider = value.into_provider();
        let path_provider = provider.flat_map(|p: Pa| Serializable::new(InputFiles::new(p)));
        prop.set_with(path_provider)?;
        self.inputs.push_with(prop);
        Ok(())
    }

    pub fn add_input_prop<T: Serialize + Send + Sync + Clone + 'static, P>(
        &mut self,
        prop: &P,
    ) -> ProjectResult
    where
        P: IntoProvider<T> + Clone,
        <P as IntoProvider<T>>::Provider: 'static,
    {
        let prop = prop.clone().into_provider();
        let mut string_prov = AnonymousProvider::new(prop.flat_map(Serializable::new));
        self.inputs.push_with(string_prov);
        Ok(())
    }

    pub fn get_input(&self) -> ProjectResult<&Input> {
        self.final_input.get_or_try_init(|| {
            let inputs = self.inputs.fallible_get()?;
            let input = Input::new(&self.task_id, inputs);
            Ok(input)
        })
    }

    /// Add some output file collection. Can add outputs until [`get_output`](WorkHandler::get_output) is called.
    pub fn add_output<F: FileCollection>(&mut self, fc: F) {
        *self.outputs.get_or_insert(FileSet::new()) += FileSet::from_iter(fc.files());
    }

    /// Add some output file collection. Can add outputs until [`get_output`](WorkHandler::get_output) is called.
    pub fn add_output_provider<P, F>(&mut self, fc_provider: P)
    where
        P: Provider<F> + 'static,
        F: FileCollection + Send + Sync + Clone + 'static,
    {
        *self.outputs.get_or_insert(FileSet::new()) += FileSet::with_provider(fc_provider);
    }

    /// Add data that can be serialized, then deserialized later for reuse
    pub fn add_serialized_data<P, T: Serialize + DeserializeOwned + 'static + Send + Sync + Clone>(
        &mut self,
        id: &str,
        value: P,
    ) where
        P: IntoProvider<T>,
        P::Provider: 'static,
    {
        let mapped = value
            .into_provider()
            .flat_map(|s| Serializable::new(s).ok());

        self.serialized_output
            .insert(id.to_string(), AnonymousProvider::new(mapped));
    }

    /// Add data that can be serialized, then deserialized later for reuse
    pub fn add_empty_serialized_data(&mut self, id: &str) {
        self.serialized_output.insert(
            id.to_string(),
            AnonymousProvider::new(provider!(|| Serializable::new(()).unwrap())),
        );
    }

    /// Get the output of this file collection
    pub fn get_output(&self) -> ProjectResult<Option<&Output>> {
        self.final_output
            .get_or_try_init(|| -> ProjectResult<Option<Output>> {
                let mut serialized = HashMap::new();

                for (key, data) in &self.serialized_output {
                    serialized.insert(key.clone(), data.fallible_get()?);
                }

                Ok(self
                    .outputs
                    .as_ref()
                    .map(|o| Output::new(o.clone(), serialized.clone()))
                    .or_else(|| {
                        if serialized.is_empty() {
                            Some(Output::new(FileSet::new(), serialized.clone()))
                        } else {
                            None
                        }
                    }))
            })
            .map(|o| o.as_ref())
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

    fn serialize_data<T: Serialize>(val: T) -> impl Provider<String> {
        let string = serializer::to_string(&val).ok();
        // let owned = Arc::new(val) as Arc<dyn Serialize>;
        provider!(move || { string.clone() })
    }
}

impl IntoBuildable for &WorkHandler {
    type Buildable = VecProp<Serializable>;

    fn into_buildable(self) -> Self::Buildable {
        // let mut container = BuiltByContainer::new();
        // for i in &self.inputs {
        //     container.add(i.clone());
        // }
        // container
        self.inputs.clone().into_buildable()
    }
}

/// An input file is used to serialize a path
#[derive(Debug)]
pub struct InputFile(PathBuf);

impl InputFile {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        Self(path)
    }

    /// Direct implementaiton of serialize
    pub fn serialize<P: AsRef<Path>, S: Serializer>(
        path: P,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        Self::new(path).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<PathBuf, D::Error> {
        let data = InputFileData::deserialize(deserializer)?;
        Ok(data.path)
    }
}

#[derive(Serialize, Deserialize)]
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

/// Represents change from previous run
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

/// Used to serialize a fileset
pub struct InputFiles(FileSet);

impl InputFiles {
    fn new<F: FileCollection>(fc: F) -> Self {
        let fileset = FileSet::from_iter(fc.files());
        Self(fileset)
    }
}

impl Serialize for InputFiles {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let files = self.0.files();
        if !files.is_empty() {
            let data = InputFilesData::new(self.0.clone());
            data.serialize(serializer)
        } else {
            ().serialize(serializer)
        }
    }
}

#[derive(Debug, Serialize)]
struct InputFilesData {
    all_files: HashSet<PathBuf>,
    data: HashMap<PathBuf, InputFile>,
}

impl InputFilesData {
    pub fn new(set: FileSet) -> Self {
        let files = set.files();
        Self {
            all_files: files.clone(),
            data: files
                .into_iter()
                .map(|f| (f.clone(), InputFile::new(f)))
                .collect(),
        }
    }
}
