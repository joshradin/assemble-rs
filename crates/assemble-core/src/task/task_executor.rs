use crate::identifier::TaskId;
use crate::project::{Project, SharedProject};
use crate::task::executable::Executable;
use crate::task::task_executor::hidden::TaskWork;
use crate::task::ExecutableTask;
use crate::utilities::ArcExt;
use crate::work_queue::{TypedWorkerQueue, WorkToken, WorkTokenBuilder, WorkerExecutor};
use crate::BuildResult;
use std::any::Any;
use std::sync::{Arc, LockResult, RwLock};
use std::vec::Drain;
use std::{io, thread};

/// The task executor. Implemented on top of a thread pool to maximize parallelism.
pub struct TaskExecutor<'exec> {
    task_queue: TypedWorkerQueue<'exec, TaskWork>,
    project: SharedProject,
    task_returns: Arc<RwLock<Vec<(TaskId, BuildResult<(bool, bool)>)>>>,
}

impl<'exec> TaskExecutor<'exec> {
    /// Create a new task executor
    pub fn new(project: SharedProject, executor: &'exec WorkerExecutor) -> Self {
        let mut typed_queue = executor.queue().typed();
        Self {
            task_queue: typed_queue,
            project,
            task_returns: Default::default(),
        }
    }

    /// Queue a task to be executed
    pub fn queue_task<E: ExecutableTask + 'static>(&mut self, task: E) -> io::Result<()> {
        let token = TaskWork::new(Box::new(task), &self.project, &self.task_returns);
        let _ = self.task_queue.submit(token)?;
        Ok(())
    }

    /// Gets finished tasks along with their build result. Does not repeat outputs, so the returned
    /// vector must be used
    #[must_use]
    pub fn finished_tasks(&mut self) -> Vec<(TaskId, BuildResult<(bool, bool)>)> {
        let mut guard = self.task_returns.write().expect("Panicked at a bad time");
        guard.drain(..).collect()
    }

    /// Wait for all running and queued tasks to finish.
    pub fn finish(
        mut self,
    ) -> (
        Vec<(TaskId, BuildResult<(bool, bool)>)>,
        Option<Box<dyn Any + Send + 'static>>,
    ) {
        let error = self.task_queue.join().err();
        match Arc::try_unwrap(self.task_returns) {
            Ok(returns) => {
                let returns = returns
                    .write()
                    .expect("returns poisoned")
                    .drain(..)
                    .collect::<Vec<_>>();
                (returns, error)
            }
            _ => {
                unreachable!("Since all references should be weak, this shouldn't be possible")
            }
        }
    }
}

/// Hides implementation details for TaskWork
mod hidden {
    use super::*;
    use crate::logging::LOGGING_CONTROL;
    use crate::utilities::try_;
    use crate::work_queue::ToWorkToken;
    use std::sync::{Mutex, Weak};
    use std::thread;
    use std::time::Instant;

    pub struct TaskWork {
        exec: Box<dyn ExecutableTask>,
        project: Weak<RwLock<Project>>,
        return_vec: Arc<RwLock<Vec<(TaskId, BuildResult<(bool, bool)>)>>>,
    }

    impl TaskWork {
        pub fn new(
            exec: Box<dyn ExecutableTask>,
            project: &SharedProject,
            return_vec: &Arc<RwLock<Vec<(TaskId, BuildResult<(bool, bool)>)>>>,
        ) -> Self {
            Self {
                exec,
                project: project.weak(),
                return_vec: return_vec.clone(),
            }
        }
    }

    impl ToWorkToken for TaskWork {
        fn on_start(&self) -> Box<dyn Fn() + Send + Sync> {
            let id = self.exec.task_id().clone();
            Box::new(move || {
                LOGGING_CONTROL.start_task(&id);
                LOGGING_CONTROL.in_task(id.clone());
                trace!("{} starting task {}", thread::current().name().unwrap(), id);
            })
        }

        fn on_complete(&self) -> Box<dyn Fn() + Send + Sync> {
            let id = self.exec.task_id().clone();
            Box::new(move || {
                trace!("{} finished task {}", thread::current().name().unwrap(), id);
                LOGGING_CONTROL.end_task(&id);
                LOGGING_CONTROL.reset();
            })
        }

        fn work(mut self) {
            let upgraded_project = self
                .project
                .upgrade()
                .expect("Project dropped but task attempting to be ran");
            let project = upgraded_project.read().unwrap();
            let output = { self.exec.execute(&*project) };
            let up_to_date = self.exec.task_up_to_date();
            let did_work = self.exec.did_work();
            let mut write_guard = self
                .return_vec
                .write()
                .expect("Couldn't get access to return vector");

            let status = (self.exec.task_id().clone(), output.map(|_| (up_to_date, did_work)));
            write_guard.push(status);
        }
    }
}
