//! Traits and struct that help dictate the building of binaries

pub struct Workspace {}

/// Need to create this to make tasks
#[derive(Debug)]
pub struct TaskSpec {
    task_identifier: String,
    task_type: String,
    do_first: Vec<String>,
    do_last: Vec<String>,
}

#[derive(Debug, Default)]
pub struct TaskSpecBuilder {
    task_identifier: String,
    task_type: Option<String>,
    do_first: Vec<String>,
    do_last: Vec<String>,
}

impl TaskSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn task_id(mut self, id: String) -> Self {
        self.task_identifier = id;
        self
    }

    fn _task_type(&self) -> String {
        self.task_type.clone().unwrap_or(String::from("Empty"))
    }

    pub fn task_type(mut self, task_type: String) -> Self {
        self.task_type = Some(task_type);
        self
    }

    pub fn first(mut self, action: String) -> Self {
        self.do_first.push(action);
        self
    }

    pub fn last(mut self, action: String) -> Self {
        self.do_last.push(action);
        self
    }

    pub fn build(self) -> TaskSpec {
        let task_type = self._task_type();
        TaskSpec {
            task_identifier: self.task_identifier,
            task_type,
            do_first: self.do_first,
            do_last: self.do_last,
        }
    }
}

#[derive(Debug)]
pub struct ProjectSpec {
    tasks: Vec<TaskSpec>,
}
