//! Turns a list of strings into a task request object

use crate::error::PayloadError;
use crate::identifier::TaskId;
use crate::project::error::{ProjectError, ProjectResult};
use crate::project::SharedProject;
use crate::task::flags::{OptionsSlurper, WeakOptionsDecoder};
use crate::task::task_container::FindTask;
use std::collections::{HashMap, VecDeque};

/// The finalized tasks requests.
#[derive(Debug)]
pub struct TaskRequests {
    task_to_weak_decoder: HashMap<TaskId, usize>,
    weak_decoders: Vec<WeakOptionsDecoder>,
    tasks: Vec<TaskId>,
}

impl TaskRequests {
    /// Build a task requests object from a given project and an arbitrary list of strings that contain
    /// only tasks, options, and values for said options if required.
    pub fn build<I: IntoIterator<Item = S>, S: AsRef<str>>(
        project: &SharedProject,
        args: I,
    ) -> ProjectResult<Self> {
        let mut builder = TaskRequestsBuilder::new();
        let mut reqs = VecDeque::from_iter(args.into_iter().map(|s| s.as_ref().to_string()));

        if reqs.is_empty() {
            // first run, add defaults if they exist.
            project.with(|p| {
                reqs.extend(p.default_tasks().iter().map(|s| s.to_string()));
            })
        }

        if reqs.is_empty() {
            return Ok(builder.finish());
        }

        while let Some(task) = reqs.pop_front() {
            let task_req = task.as_ref();

            let ids: Option<Vec<TaskId>> = project.find_eligible_tasks(task_req)?;

            if let Some(ids) = ids {
                let first = ids.first().unwrap();
                let mut any_handle = project.get_task(first)?;
                let resolved = any_handle.resolve_shared(project)?;

                if let Some(ops) = resolved.options_declarations() {
                    let slurper = OptionsSlurper::new(&ops);
                    let slice = reqs.make_contiguous();
                    let (weak, count) = slurper.slurp(slice).map_err(PayloadError::new)?;
                    builder.add_configured_tasks(ids, weak);
                    reqs.drain(..count);
                } else {
                    builder.add_tasks(ids);
                }
            } else {
                return Err(ProjectError::NoIdentifiersFound(task_req.to_string()).into());
            }
        }

        Ok(builder.finish())
    }

    /// Get the decoder for a given task id
    pub fn decoder(&self, id: &TaskId) -> Option<WeakOptionsDecoder> {
        let index = self.task_to_weak_decoder.get(id)?;
        Some(self.weak_decoders[*index].clone())
    }

    /// Tasks requests, in-order
    pub fn requested_tasks(&self) -> &[TaskId] {
        &self.tasks[..]
    }
}

struct TaskRequestsBuilder {
    in_progress: TaskRequests,
}

impl TaskRequestsBuilder {
    fn new() -> Self {
        Self {
            in_progress: TaskRequests {
                task_to_weak_decoder: Default::default(),
                weak_decoders: vec![],
                tasks: vec![],
            },
        }
    }

    fn finish(self) -> TaskRequests {
        self.in_progress
    }

    fn add_tasks<I>(&mut self, tasks: I)
    where
        I: IntoIterator<Item = TaskId>,
    {
        self.in_progress.tasks.extend(tasks)
    }

    fn add_configured_tasks<I>(&mut self, tasks: I, decoder: WeakOptionsDecoder)
    where
        I: IntoIterator<Item = TaskId>,
    {
        let index = self.in_progress.weak_decoders.len();
        self.in_progress.weak_decoders.push(decoder);
        for task in tasks {
            self.in_progress.tasks.push(task.clone());
            self.in_progress.task_to_weak_decoder.insert(task, index);
        }
    }
}
