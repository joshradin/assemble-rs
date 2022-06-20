use crate::{ExecutableTask, Project};
use crate::workqueue::{TypedWorkerQueue, WorkToken, WorkTokenBuilder};

pub struct TaskExecutor<'exec, T : ExecutableTask> {
    task_queue: TypedWorkerQueue<'exec, fn()>,
    project: Project<T>
}
