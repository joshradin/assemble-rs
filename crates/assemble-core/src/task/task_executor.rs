use crate::identifier::TaskId;
use crate::project::Project;

use crate::task::task_executor::hidden::TaskWork;
use crate::task::ExecutableTask;

use crate::work_queue::{TypedWorkerQueue, WorkerExecutor};
use crate::BuildResult;
use std::any::Any;
use std::sync::Arc;

use crate::project::finder::{ProjectFinder, ProjectPath, ProjectPathBuf};
use crate::project::shared::SharedProject;
use parking_lot::RwLock;
use std::io;

/// The task executor. Implemented on top of a thread pool to maximize parallelism.
pub struct TaskExecutor<'exec> {
    task_queue: TypedWorkerQueue<'exec, TaskWork>,
    project: SharedProject,
    task_returns: Arc<RwLock<Vec<(TaskId, BuildResult<(bool, bool)>)>>>,
}

impl<'exec> TaskExecutor<'exec> {
    /// Create a new task executor
    pub fn new(project: SharedProject, executor: &'exec WorkerExecutor) -> Self {
        let typed_queue = executor.queue().typed();
        Self {
            task_queue: typed_queue,
            project,
            task_returns: Default::default(),
        }
    }

    /// Queue a task to be executed
    pub fn queue_task<E: ExecutableTask + 'static>(&mut self, task: E) -> io::Result<()> {
        let project = task
            .task_id()
            .project_id()
            .expect("project id should always exist at this point");
        trace!(
            "finding project {} to execute {} with",
            project,
            task.task_id()
        );

        let finder = ProjectFinder::new(&self.project);
        let project = finder
            .find(&ProjectPathBuf::from(project))
            .expect("should exist");

        let token = TaskWork::new(Box::new(task), &project, &self.task_returns);
        let _ = self.task_queue.submit(token)?;
        Ok(())
    }

    /// Gets finished tasks along with their build result. Does not repeat outputs, so the returned
    /// vector must be used
    #[must_use]
    pub fn finished_tasks(&mut self) -> Vec<(TaskId, BuildResult<(bool, bool)>)> {
        let mut guard = self.task_returns.write();
        guard.drain(..).collect()
    }

    /// Wait for all running and queued tasks to finish.
    pub fn finish(
        self,
    ) -> (
        Vec<(TaskId, BuildResult<(bool, bool)>)>,
        Option<Box<dyn Any + Send + 'static>>,
    ) {
        let error = self.task_queue.join().err();
        match Arc::try_unwrap(self.task_returns) {
            Ok(returns) => {
                let returns = returns.write().drain(..).collect::<Vec<_>>();
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

    use crate::project::shared::WeakSharedProject;
    use crate::work_queue::ToWorkToken;
    use std::sync::Weak;
    use std::thread;

    pub struct TaskWork {
        exec: Box<dyn ExecutableTask>,
        project: WeakSharedProject,
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
            let id = self.exec.task_id();
            Box::new(move || {
                LOGGING_CONTROL.start_task(&id);
                LOGGING_CONTROL.in_task(id.clone());
                trace!("{} starting task {}", thread::current().name().unwrap(), id);
            })
        }

        fn on_complete(&self) -> Box<dyn Fn() + Send + Sync> {
            let id = self.exec.task_id();
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
            upgraded_project.with(|project| {
                let output = { self.exec.execute(&*project) };
                let up_to_date = self.exec.task_up_to_date();
                let did_work = self.exec.did_work();
                let mut write_guard = self.return_vec.write();

                let status = (self.exec.task_id(), output.map(|_| (up_to_date, did_work)));
                write_guard.push(status);
            })
        }
    }
}
