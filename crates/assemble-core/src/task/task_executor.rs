
use crate::task::task_executor::hidden::TaskWork;
use crate::task::TaskId;
use crate::work_queue::{TypedWorkerQueue, WorkToken, WorkTokenBuilder, WorkerExecutor};
use crate::{BuildResult, ExecutableTask, Project};
use std::io;
use std::sync::{Arc, LockResult, RwLock};
use std::vec::Drain;
use crate::utilities::ArcExt;

/// The task executor. Implemented on top of a thread pool to maximize parallelism.
pub struct TaskExecutor<'exec, E: ExecutableTask + Send + Sync + 'static> {
    task_queue: TypedWorkerQueue<'exec, TaskWork<E>>,
    project: Arc<Project<E>>,
    task_returns: Arc<RwLock<Vec<(TaskId, BuildResult)>>>,
}



impl<'exec, E: ExecutableTask> TaskExecutor<'exec, E> {
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
    pub fn finished_tasks(&mut self) -> Vec<(TaskId, BuildResult)> {
        let mut guard = self.task_returns.write().expect("Panicked at a bad time");
        guard.drain(..).collect()
    }

    /// Wait for all running and queued tasks to finish.
    #[must_use]
    pub fn finish(mut self) -> (Project<E>, Vec<(TaskId, BuildResult)>) {
        self.task_queue.join().expect("Failed to join worker tasks");
        match (Arc::try_unwrap(self.project), Arc::try_unwrap(self.task_returns)) {
            (Ok(proj), Ok(returns)) => {
                let returns = returns.write().expect("returns poisoned")
                    .drain(..)
                    .collect::<Vec<_>>();
                (proj, returns)
            }
            _ => {
                unreachable!("Since all references should be weak, this shouldn't be possible")
            }
        }
    }
}

/// Hides implementation details for TaskWork
mod hidden {
    use std::sync::Weak;
    use std::time::Instant;
    use crate::utilities::try_;
    use crate::work_queue::ToWorkToken;
    use super::*;
    pub struct TaskWork<E: ExecutableTask + Send + Sync> {
        exec: E,
        project: Weak<Project<E>>,
        return_vec: Arc<RwLock<Vec<(TaskId, BuildResult)>>>,
    }

    impl<E: ExecutableTask + Send + Sync> TaskWork<E> {
        pub fn new(exec: E,
                   project: &Arc<Project<E>>,
                   return_vec: &Arc<RwLock<Vec<(TaskId, BuildResult)>>>) -> Self {
            Self {
                exec,
                project: Arc::downgrade(project),
                return_vec: return_vec.clone(),
            }
        }
    }

    impl<E : ExecutableTask + Send + Sync + 'static> ToWorkToken for TaskWork<E> {
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
            let upgraded_project = self.project.upgrade().expect("Project dropped but task attempting to be ran");
            let project = upgraded_project.as_ref();

            let output = {
                self.exec.execute(project)
            };
            let mut write_guard = self.return_vec.write().expect("Couldn't get access to return vector");

            let status = (self.exec.task_id().clone(), output);
            write_guard.push(status);
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, Mutex};
    use crate::{Project, Task};
    use crate::task::{Action, Empty, ExecutableTaskMut, TaskId};
    use std::io::Write;
    use crate::task::task_executor::TaskExecutor;
    use crate::work_queue::WorkerExecutor;

    #[test]
    fn can_execute_task() {
        let mut task = Empty::default().into_task().unwrap();
        task.set_task_id(TaskId::new("test"));
        let mut buffer: Arc<Mutex<Vec<u8>>> = Default::default();

        let buffer_clone = buffer.clone();
        task.first(Action::new(move |_, _| {
            let buffer = buffer.clone();
            let mut guard = buffer.lock().unwrap();
            write!(guard, "Hello, World!")?;
            println!("MUM GET THE CAMERA");
            Ok(())
        }));

        let project = Project::default();

        let executor = WorkerExecutor::new(1).unwrap();

        let mut task_executor = TaskExecutor::new(project, &executor);

        task_executor.queue_task(task).expect("couldn't queue task");
        let (_, mut finished) = task_executor.finish();

        let (task_id, result) = finished.remove(0);

        assert_eq!(task_id.0, "test");
        assert!(result.is_ok());

        let lock = buffer_clone.lock().unwrap();
        let string = String::from_utf8(lock.clone()).unwrap();
        assert_eq!(string, "Hello, World!");
    }
}
