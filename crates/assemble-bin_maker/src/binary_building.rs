//! Traits and struct that help dictate the building of binaries

pub struct Workspace {}

/// Need to create this to make tasks
pub struct TaskSpec {
    task_identifier: String,
    task_type: String,
}

#[derive(Debug, Default)]
pub struct TaskSpecBuilder {
    task_identifier: String,
    task_type: Option<String>
}

impl TaskSpecBuilder {

    pub fn new() -> Self {
        Self::default()
    }

    pub fn task_type(&self) -> String {
        self.task_type.clone().unwrap_or(String::from("Empty"))
    }


    pub fn build(self) -> TaskSpec {
        let task_type = self.task_type();
        TaskSpec {
            task_identifier: self.task_identifier,
            task_type
        }
    }
}
