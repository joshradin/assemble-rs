use crate::identifier::TaskId;
use crate::project::{Project, SharedProject};
use crate::task::executable::Executable;
use crate::task::task_executor::hidden::TaskWork;
use crate::task::ExecutableTask;
use crate::utilities::ArcExt;
use crate::work_queue::{TypedWorkerQueue, WorkToken, WorkTokenBuilder, WorkerExecutor};
use crate::BuildResult;
use std::io;
use std::sync::{Arc, LockResult, RwLock};
use std::vec::Drain;

/// The task executor. Implemented on top of a thread pool to maximize parallelism.
pub struct TaskExecutor<'exec> {
    task_queue: TypedWorkerQueue<'exec, TaskWork>,
    project: SharedProject,
    task_returns: Arc<RwLock<Vec<(TaskId, BuildResult)>>>,
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
    pub fn finished_tasks(&mut self) -> Vec<(TaskId, BuildResult)> {
        let mut guard = self.task_returns.write().expect("Panicked at a bad time");
        guard.drain(..).collect()
    }

    /// Wait for all running and queued tasks to finish.
    #[must_use]
    pub fn finish(mut self) -> Vec<(TaskId, BuildResult)> {
        self.task_queue.join().expect("Failed to join worker tasks");
        match Arc::try_unwrap(self.task_returns) {
            Ok(returns) => {
                let returns = returns
                    .write()
                    .expect("returns poisoned")
                    .drain(..)
                    .collect::<Vec<_>>();
                returns
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
    use crate::utilities::try_;
    use crate::work_queue::ToWorkToken;
    use std::sync::{Mutex, Weak};
    use std::time::Instant;
    pub struct TaskWork {
        exec: Box<dyn ExecutableTask>,
        project: Weak<RwLock<Project>>,
        return_vec: Arc<RwLock<Vec<(TaskId, BuildResult)>>>,
    }

    impl TaskWork {
        pub fn new(
            exec: Box<dyn ExecutableTask>,
            project: &SharedProject,
            return_vec: &Arc<RwLock<Vec<(TaskId, BuildResult)>>>,
        ) -> Self {
            Self {
                exec,
                project: Arc::downgrade(&project.0),
                return_vec: return_vec.clone(),
            }
        }
    }

    impl ToWorkToken for TaskWork {
        fn on_start(&self) -> Box<dyn Fn() + Send + Sync> {
            let id = self.exec.task_id().clone();
            Box::new(move || {
                println!("Executing task = {:?}", id);
            })
        }

        fn on_complete(&self) -> Box<dyn Fn() + Send + Sync> {
            let id = self.exec.task_id().clone();
            Box::new(move || {
                println!("Finished task = {:?}", id);
            })
        }

        fn work(mut self) {
            let upgraded_project = self
                .project
                .upgrade()
                .expect("Project dropped but task attempting to be ran");
            let project = upgraded_project.read().unwrap();
            let output = { self.exec.execute(&*project) };
            let mut write_guard = self
                .return_vec
                .write()
                .expect("Couldn't get access to return vector");

            let status = (self.exec.task_id().clone(), output);
            write_guard.push(status);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::defaults::tasks::Empty;
    use crate::identifier::TaskId;
    use crate::task::task_executor::TaskExecutor;
    use crate::task::Action;
    use crate::work_queue::WorkerExecutor;
    use crate::{Executable, Project, Task};
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    #[test]
    fn can_execute_task() {
        let project = Project::new().unwrap();
        let mut task = Executable::new(
            project.clone(),
            Empty::default(),
            TaskId::new("test").unwrap(),
        );
        let mut buffer: Arc<Mutex<Vec<u8>>> = Default::default();

        let buffer_clone = buffer.clone();
        task.do_first(move |exec, _| {
            let buffer = buffer.clone();
            let mut guard = buffer.lock().unwrap();
            write!(guard, "Hello, World!")?;
            println!("MUM GET THE CAMERA");
            Ok(())
        })
        .unwrap();

        let executor = WorkerExecutor::new(1).unwrap();

        let mut task_executor = TaskExecutor::new(project, &executor);

        task_executor.queue_task(task).expect("couldn't queue task");
        let mut finished = task_executor.finish();

        let (task_id, result) = finished.remove(0);

        assert_eq!(task_id, "test");
        assert!(result.is_ok());

        let lock = buffer_clone.lock().unwrap();
        let string = String::from_utf8(lock.clone()).unwrap();
        assert_eq!(string, "Hello, World!");
    }
}
