use crate::__export::{Serializable, TaskId};

use std::time::SystemTime;

/// Represents some previous work
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Input {
    task_id: TaskId,
    timestamp: SystemTime,
    serialized_data: Vec<Serializable>,
}

impl Input {
    pub fn new<'a>(id: &TaskId, data: Vec<Serializable>) -> Self {
        let now = SystemTime::now();
        Self {
            task_id: id.clone(),
            timestamp: now,
            serialized_data: data,
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
