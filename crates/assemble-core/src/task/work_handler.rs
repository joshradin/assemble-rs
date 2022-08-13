use crate::identifier::{Id, TaskId};
use crate::project::ProjectResult;
use crate::properties::{IntoProvider, Prop, Provides, ProvidesExt};
use crate::task::up_to_date::UpToDate;
use crate::Project;
use input::Input;
use log::{info, trace};
use once_cell::sync::{Lazy, OnceCell};
use ron::ser::PrettyConfig;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
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
    final_input: OnceCell<Input>,
    up_to_date_status: OnceCell<bool>,
    did_work: bool,
}

impl WorkHandler {
    pub fn new(id: &TaskId, cache_loc: PathBuf) -> Self {
        Self {
            task_id: id.clone(),
            cache_location: cache_loc,
            inputs: vec![],
            final_input: OnceCell::new(),
            up_to_date_status: OnceCell::new(),
            did_work: true,
        }
    }

    pub fn remove_stored_input(&self) -> io::Result<()> {
        let path = self.task_id.as_path();
        let file_location = self.cache_location.join(path);
        if file_location.exists() {
            std::fs::remove_file(file_location)?;
        }
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

    pub fn try_get_prev_input(&self) -> Option<Input> {
        let path = self.task_id.as_path();
        let file_location = self.cache_location.join(path);
        if file_location.exists() {
            let mut read = File::open(&file_location).ok()?;
            let mut buffer = String::new();
            read.read_to_string(&mut buffer)
                .expect(&format!("Could not read to end of {:?}", file_location));
            ron::from_str(&buffer).ok()
        } else {
            None
        }
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

    pub fn add_prop<T: Serialize + Send + Sync + Clone + 'static>(
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

    fn serialize_data<T: Serialize>(val: T) -> impl Provides<String> {
        ron::to_string(&val)
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
}

pub struct InputFile(PathBuf);

impl InputFile {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        Self(path)
    }
}

impl Serialize for InputFile {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.0.exists() {
            let mut read =
                File::open(&self.0).expect(&format!("Dont have read access to {:?}", self.0));
            let mut buffer = Vec::new();
            read.read_to_end(&mut buffer)
                .expect(&format!("Could not read to end of {:?}", self.0));
            buffer.serialize(serializer)
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
