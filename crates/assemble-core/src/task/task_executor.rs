
use crate::task::task_executor::hidden::TaskWork;
use crate::task::TaskIdentifier;
use crate::workqueue::{TypedWorkerQueue, WorkToken, WorkTokenBuilder, WorkerExecutor};
use crate::{BuildResult, ExecutableTask, Project};
use std::io;
use std::sync::{Arc, LockResult, RwLock};
use std::vec::Drain;
use crate::utilities::ArcExt;

/// The task executor. Implemented on top of a thread pool to maximize parallelism.
pub struct TaskExecutor<'exec, E: ExecutableTask + Send + Sync + 'static> {
    task_queue: TypedWorkerQueue<'exec, TaskWork<E>>,
    project: Arc<Project<E>>,
    task_returns: Arc<RwLock<Vec<(TaskIdentifier, BuildResult)>>>,
}

impl<'exec, E: ExecutableTask + Send + Sync> TaskExecutor<'exec, E> {
    /// Create a new task executor
    pub fn new(project: Project<E>, executor: &'exec WorkerExecutor) -> Self {
        let mut typed_queue = executor.queue().typed();
        Self {
            task_queue: typed_queue,
            project: Arc::new(project),
            task_returns: Default::default(),
        }
    }

    /// Queue a task to be executed
    pub fn queue_task(&mut self, task: E) -> io::Result<()> {
        let token = TaskWork::new(task, &self.project,&self.task_returns);
        let _ = self.task_queue.submit(token)?;
        Ok(())
    }

    /// Gets finished tasks along with their build result. Does not repeat outputs, so the returned
    /// vector must be used
    #[must_use]
    pub fn finished_tasks(&mut self) -> Vec<(TaskIdentifier, BuildResult)> {
        let mut guard = self.task_returns.write().expect("Panicked at a bad time");
        guard.drain(..).collect()
    }

    /// Wait for all running and queued tasks to finish.
    pub fn finish(self) -> Project<E> {
        match Arc::try_unwrap(self.project) {
            Ok(o) => {o}
            Err(_) => {
                unreachable!("Since all references should be weak, this shouldn't be possible")
            }
        }
    }
}

/// Hides implementation details for TaskWork
mod hidden {
    use std::sync::Weak;
    use crate::workqueue::ToWorkToken;
    use super::*;
    pub struct TaskWork<E: ExecutableTask + Send + Sync> {
        exec: E,
        project: Weak<Project<E>>,
        return_vec: Arc<RwLock<Vec<(TaskIdentifier, BuildResult)>>>,
    }

    impl<E: ExecutableTask + Send + Sync> TaskWork<E> {
        pub fn new(exec: E,
                   project: &Arc<Project<E>>,
                   return_vec: &Arc<RwLock<Vec<(TaskIdentifier, BuildResult)>>>) -> Self {
            Self {
                exec,
                project: Arc::downgrade(project),
                return_vec: return_vec.clone(),
            }
        }
    }

    impl<E : ExecutableTask + Send + Sync + 'static> ToWorkToken for TaskWork<E> {
        fn on_start(&self) -> fn() {
            todo!()
        }

        fn on_complete(&self) -> fn() {
            todo!()
        }

        fn work(self) {

        }
    }
}
