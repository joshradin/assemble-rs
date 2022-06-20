use crate::{ExecutableTask, Project};
use crate::workqueue::{TypedWorkerQueue, WorkToken, WorkTokenBuilder};

pub struct TaskExecutor<'exec, T : ExecutableTask> {
    task_queue: TypedWorkerQueue<'exec, fn()>,
    project: Project<T>
}


pub struct ExecutableTaskWork<'proj, T : ExecutableTask> {
    task: T,
    project: &'proj Project<T>
}

impl<'proj, T: ExecutableTask> ExecutableTaskWork<'proj, T> {
    pub fn new(task: T, project: &'proj Project<T>) -> Self {
        Self { task, project }
    }
}

impl<T : ExecutableTask + Send + 'static> From<ExecutableTaskWork<'_, T>> for WorkToken {
    fn from(e: ExecutableTaskWork<'_, T>) -> Self {
        let action = move || {
            let mut task = e.task;
            task.execute(e.project);
        };

        WorkTokenBuilder::new(action).build()
    }
}