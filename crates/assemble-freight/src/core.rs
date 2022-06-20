use std::io;
use std::num::NonZeroUsize;
use assemble_core::{ExecutableTask, Project};
use assemble_core::work_queue::WorkerExecutor;

mod task_resolver;
pub use task_resolver::TaskResolver;

/// Initialize the task executor.
pub fn init_executor(num_workers: NonZeroUsize) -> io::Result<WorkerExecutor> {
    let num_workers = num_workers.get();

    WorkerExecutor::new(num_workers)
}


