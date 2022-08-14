use crate::__export::{Provides, TaskId};
use crate::identifier::Id;
use crate::properties::Prop;
use std::collections::HashMap;
use std::time::SystemTime;

/// Represents some previous work
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Input {
    task_id: TaskId,
    timestamp: SystemTime,
    serialized_data: HashMap<Id, String>,
}

impl Input {
    pub fn new<'a>(id: &TaskId, data: impl IntoIterator<Item = &'a Prop<String>>) -> Self {
        let serialized = data
            .into_iter()
            .map(|prop: &'a Prop<String>| {
                let id = prop.id().clone();
                let string = prop.get();
                (id, string)
            })
            .collect::<HashMap<_, _>>();
        let now = SystemTime::now();
        Self {
            task_id: id.clone(),
            timestamp: now,
            serialized_data: serialized,
        }
    }

    /// Check whether the input has changed
    pub fn input_changed(&self, prev: Option<&Input>) -> bool {
        if let Some(prev) = prev {
            if self.task_id != prev.task_id {
                panic!("Cant compare inputs of different tasks");
            }
            trace!(
                "comparing {:#?} to {:#?}",
                self.serialized_data,
                prev.serialized_data
            );
            let input_data_same = self.serialized_data == prev.serialized_data;
            !input_data_same
        } else {
            true
        }
    }

    /// Whether any inputs have been declared for this task
    pub fn any_inputs(&self) -> bool {
        !self.serialized_data.is_empty()
    }
}
