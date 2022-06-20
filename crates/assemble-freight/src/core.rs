use std::io;
use assemble_core::work_queue::WorkerExecutor;

/// Initialize the task executor.
///
/// # Panic
/// Will panic if `num_workers` is 0.
pub fn initialize_executor(num_workers: usize) -> io::Result<WorkerExecutor> {
    if num_workers == 0 {
        panic!("The number of workers can not be 0");
    }

    WorkerExecutor::new(num_workers)
}