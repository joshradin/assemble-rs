//! The work queue allows for submission and completion of work.
//!
//! There are two ways of interacting with the worker executor.
//! 1. By directly interfacing with [`WorkerExecutor`](WorkerExecutor) instance.
//! 2. Using a [`WorkerQueue`](WorkerQueue), which allows for easy handling of multiple requests.

use crate::file_collection::Component::Path;
use crate::project::ProjectError;
use crossbeam::channel::{bounded, unbounded, Receiver, SendError, Sender, TryRecvError};
use crossbeam::deque::{Injector, Steal, Stealer, Worker};
use crossbeam::scope;
use crossbeam::thread::ScopedJoinHandle;
use std::collections::HashMap;
use std::error::Error;
use std::io::ErrorKind;
use std::marker::PhantomData;
use std::panic::catch_unwind;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;
use std::{io, thread};
use uuid::Uuid;

/// A Work Token is a single unit of work done within the Work Queue. Can be built using a [WorkTokenBuilder](WorkTokenBuilder)
pub struct WorkToken {
    pub on_start: Box<dyn Fn() + Send + 'static>,
    pub on_complete: Box<dyn Fn() + Send + 'static>,
    pub work: Box<dyn FnOnce() + Send + 'static>,
}

impl WorkToken {
    fn new(
        on_start: Box<dyn Fn() + Send + 'static>,
        on_complete: Box<dyn Fn() + Send + 'static>,
        work: Box<dyn FnOnce() + Send + 'static>,
    ) -> Self {
        Self {
            on_start,
            on_complete,
            work,
        }
    }
}

pub trait ToWorkToken: Send + Sync + 'static
{
    fn on_start(&self) -> fn();
    fn on_complete(&self) -> fn();
    fn work(self);
}

impl<T: ToWorkToken> From<T> for WorkToken
{
    fn from(tok: T) -> Self {
        let on_start = tok.on_start();
        let on_complete = tok.on_complete();
        WorkTokenBuilder::new(|| tok.work())
            .on_start(on_start)
            .on_complete(on_complete)
            .build()
    }
}

impl<F: FnOnce() + Send + Sync + 'static> ToWorkToken for F {
    fn on_start(&self) -> fn() {
        || { }
    }

    fn on_complete(&self) -> fn() {
        || { }
    }

    fn work(self) {
        (self)()
    }


}

fn empty() {}

/// Builds [`WorkToken`s](WorkToken) for the work queue. Both on_start and on_complete are optional.
///
/// # Example
/// ```rust
/// # use assemble_core::workqueue::{WorkToken, WorkTokenBuilder};
/// let token: WorkToken = WorkTokenBuilder::new(|| { }).build(); // valid
/// let token: WorkToken = WorkTokenBuilder::new(|| { })
///     .on_complete(|| { })
///     .on_start(|| { })
///     .build()
///     ;
/// ```
pub struct WorkTokenBuilder<W, S, C>
where
    W: FnOnce(),
{
    on_start: S,
    on_complete: C,
    work: W,
}

impl<W, S, C> WorkTokenBuilder<W, S, C>
where
    W: FnOnce() + Send + 'static,
    S: Fn() + Send + 'static,
    C: Fn() + Send + 'static,
{
    pub fn build(self) -> WorkToken {
        WorkToken::new(
            Box::new(self.on_start),
            Box::new(self.on_complete),
            Box::new(self.work),
        )
    }
}

impl<W> WorkTokenBuilder<W, fn(), fn()>
where
    W: FnOnce() + Send + 'static,
{
    /// Create a new [`WorkTokenBuilder`](WorkTokenBuilder) to construct new [`WorkToken`s](WorkToken)
    pub fn new(work: W) -> Self {
        Self {
            on_start: empty,
            on_complete: empty,
            work,
        }
    }
}

impl<W, S1, C> WorkTokenBuilder<W, S1, C>
where
    W: FnOnce(),
{
    pub fn on_start<S2: Fn() + Send + 'static>(
        mut self,
        on_start: S2,
    ) -> WorkTokenBuilder<W, S2, C> {
        WorkTokenBuilder {
            on_start,
            on_complete: self.on_complete,
            work: self.work,
        }
    }
}

impl<W, S, C1> WorkTokenBuilder<W, S, C1>
where
    W: FnOnce(),
{
    pub fn on_complete<C2: Fn() + Send + 'static>(
        mut self,
        on_complete: C2,
    ) -> WorkTokenBuilder<W, S, C2> {
        WorkTokenBuilder {
            on_complete,
            on_start: self.on_start,
            work: self.work,
        }
    }
}

enum WorkerQueueRequest {
    GetStatus,
}

enum WorkerQueueResponse {
    Status(HashMap<Uuid, WorkerStatus>),
}

#[derive(Debug, Eq, PartialEq)]
enum WorkerMessage {
    Stop,
}

type WorkTokenId = u64;

/// A worker queue allows for the submission of work to be done in parallel.
pub struct WorkerExecutor {
    max_jobs: usize,
    injector: Arc<Injector<WorkerTuple>>,
    connection: Option<Connection>,
}

struct Connection {
    join_send: Sender<()>,
    inner_handle: JoinHandle<()>,

    request_sender: Sender<WorkerQueueRequest>,
    response_receiver: Receiver<WorkerQueueResponse>,
}

impl Connection {
    fn handle_request(&self, request: WorkerQueueRequest) -> WorkerQueueResponse {
        self.request_sender.send(request).unwrap();
        self.response_receiver.recv().unwrap()
    }
}

impl Drop for WorkerExecutor {
    fn drop(&mut self) {
        self.join_inner();
    }
}

impl WorkerExecutor {
    pub fn new(pool_size: usize) -> io::Result<Self> {
        let mut out = Self {
            max_jobs: pool_size,
            injector: Arc::new(Injector::new()),
            connection: None,
        };
        out.start()?;
        Ok(out)
    }

    /// Can be used to restart a joined worker queue
    fn start(&mut self) -> io::Result<()> {
        self.connection = Some(Inner::start(&self.injector, self.max_jobs)?);
        Ok(())
    }

    /// Waits for all workers to finish. Unlike the drop implementation, Calls [`finish_jobs`](WorkerQueue::finish_jobs).
    pub fn join(mut self) -> Result<(), ProjectError> {
        self.finish_jobs()?;
        self.join_inner()?;
        Ok(())
    }

    fn join_inner(&mut self) -> thread::Result<()> {
        if let Some(connection) = std::mem::replace(&mut self.connection, None) {
            let _ = connection.join_send.send(());
            connection.inner_handle.join()?;
        };
        Ok(())
    }

    /// Forces all running tokens to end.
    /// Submit some work to the Worker Queue.
    ///
    pub fn submit<I: Into<WorkToken>>(&self, token: I) -> io::Result<WorkHandle> {
        let work_token = token.into();

        let (handle, channel) = work_channel(&self);
        let id = rand::random();
        let work_tuple = WorkerTuple(id, work_token, channel);
        self.injector.push(work_tuple);
        Ok(handle)
    }

    pub fn any_panicked(&self) -> bool {
        let status = self
            .connection
            .as_ref()
            .map(|s| s.handle_request(WorkerQueueRequest::GetStatus));
        match status {
            Some(WorkerQueueResponse::Status(status)) => {
                status.values().any(|s| s == &WorkerStatus::Panic)
            }
            None => false,
        }
    }

    /// Wait for all current jobs to finish.
    pub fn finish_jobs(&mut self) -> io::Result<()> {
        if self.connection.is_none() {
            panic!("Shouldn't be possible")
        }

        loop {
            if self.injector.is_empty() {
                break;
            }
        }

        while let Some(connection) = &self.connection {
            thread::sleep(Duration::from_millis(100));
            let status = connection.handle_request(WorkerQueueRequest::GetStatus);
            let finished = match status {
                WorkerQueueResponse::Status(s) => {
                    s.values().all(|status| status == &WorkerStatus::Idle)
                }
            };
            if finished {
                break;
            }
        }
        Ok(())
    }

    /// Create a worker queue instance.
    pub fn queue(&self) -> WorkerQueue {
        WorkerQueue::new(self)
    }
}

struct Inner {
    max_jobs: usize,
    injector: Arc<Injector<WorkerTuple>>,
    worker: Worker<WorkerTuple>,
    message_sender: Sender<WorkerMessage>,
    status_receiver: Receiver<WorkStatusUpdate>,
    stop_receiver: Receiver<()>,
    handles: Vec<JoinHandle<()>>,
    id_to_status: HashMap<Uuid, WorkerStatus>,

    request_recv: Receiver<WorkerQueueRequest>,
    response_sndr: Sender<WorkerQueueResponse>,
}

#[derive(Clone)]
pub struct WorkHandle<'exec> {
    recv: Receiver<()>,
    owner: &'exec WorkerExecutor,
}

/// Creates a work handle and it's corresponding sender
fn work_channel(exec: &WorkerExecutor) -> (WorkHandle, Sender<()>) {
    let (s, r) = bounded::<()>(1);
    (
        WorkHandle {
            recv: r,
            owner: exec,
        },
        s,
    )
}

impl WorkHandle<'_> {
    /// Joins the work handle
    pub fn join(self) -> io::Result<()> {
        if self.owner.any_panicked() {
            return Err(io::Error::new(
                ErrorKind::OutOfMemory,
                "A panic occured in the thingy",
            ));
        }
        self.recv
            .recv()
            .map_err(|e| io::Error::new(ErrorKind::BrokenPipe, e))
    }
}

mod inner_impl {
    use super::*;
    impl Inner {
        /// Create a Worker queue with a set maximum amount of workers.
        fn new(
            injector: &Arc<Injector<WorkerTuple>>,
            pool_size: usize,
            stop_recv: Receiver<()>,
        ) -> io::Result<(
            Self,
            Sender<WorkerQueueRequest>,
            Receiver<WorkerQueueResponse>,
        )> {
            let (s, r) = unbounded();
            let (s2, r2) = unbounded();

            let requests = unbounded();
            let responses = unbounded();

            let mut output = Self {
                max_jobs: pool_size,
                injector: injector.clone(),
                worker: Worker::new_fifo(),
                message_sender: s,
                status_receiver: r2,
                stop_receiver: stop_recv,
                handles: vec![],
                id_to_status: HashMap::new(),

                request_recv: requests.1,
                response_sndr: responses.0,
            };
            for _ in 0..pool_size {
                let stealer = output.worker.stealer();
                let (id, handle) = AssembleWorker::new(stealer, r.clone(), s2.clone()).start()?;
                output.id_to_status.insert(id, WorkerStatus::Unknown);
                output.handles.push(handle);
            }

            Ok((output, requests.0, responses.1))
        }

        pub fn start(
            injector: &Arc<Injector<WorkerTuple>>,
            pool_size: usize,
        ) -> io::Result<Connection> {
            let (stop_s, stop_r) = unbounded();
            let (inner, sender, recv) = Self::new(injector, pool_size, stop_r)?;

            let handle = thread::spawn(move || inner.run());

            Ok(Connection {
                join_send: stop_s,
                inner_handle: handle,
                request_sender: sender,
                response_receiver: recv,
            })
        }

        fn run(mut self) {
            loop {
                match self.stop_receiver.try_recv() {
                    Ok(()) => break,
                    Err(TryRecvError::Empty) => {}
                    Err(_) => break,
                }

                let _ = self.injector.steal_batch(&self.worker);

                self.update_worker_status();
                self.handle_requests();
            }

            for _ in &self.handles {
                self.message_sender.send(WorkerMessage::Stop);
            }
            for handle in self.handles {
                handle.join();
            }
        }

        fn update_worker_status(&mut self) {
            while let Ok(status) = self.status_receiver.try_recv() {
                self.id_to_status.insert(status.worker_id, status.status);
            }
        }

        fn handle_requests(&mut self) {
            while let Ok(req) = self.request_recv.try_recv() {
                let response = self.on_request(req);
                self.response_sndr
                    .send(response)
                    .expect("Inner still exists while Outer gone")
            }
        }

        fn on_request(&mut self, request: WorkerQueueRequest) -> WorkerQueueResponse {
            match request {
                WorkerQueueRequest::GetStatus => {
                    let map = self.id_to_status.clone();
                    WorkerQueueResponse::Status(map)
                }
            }
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum WorkerStatus {
    Unknown,
    TaskRunning(WorkTokenId),
    Idle,
    Panic,
}

struct WorkStatusUpdate {
    worker_id: Uuid,
    status: WorkerStatus,
}

struct AssembleWorker {
    id: Uuid,
    stealer: Stealer<WorkerTuple>,
    message_recv: Receiver<WorkerMessage>,
    status_send: Sender<WorkStatusUpdate>,
}

impl Drop for AssembleWorker {
    fn drop(&mut self) {
        if thread::panicking() {
            self.report_status(WorkerStatus::Panic).unwrap()
        }
    }
}

impl AssembleWorker {
    pub fn new(
        stealer: Stealer<WorkerTuple>,
        message_recv: Receiver<WorkerMessage>,
        status_send: Sender<WorkStatusUpdate>,
    ) -> Self {
        let id = Uuid::new_v4();
        Self {
            id,
            stealer,
            message_recv,
            status_send,
        }
    }

    fn start(mut self) -> io::Result<(Uuid, JoinHandle<()>)> {
        let id = self.id;
        self.report_status(WorkerStatus::Idle).unwrap();
        let handle = thread::Builder::new().spawn(move || self.run())?;
        Ok((id, handle))
    }

    fn run(&mut self) {
        'outer: loop {
            match self.message_recv.try_recv() {
                Ok(msg) => match msg {
                    WorkerMessage::Stop => break 'outer,
                },
                Err(TryRecvError::Empty) => {}
                Err(_) => break 'outer,
            }

            if let Steal::Success(tuple) = self.stealer.steal() {
                let WorkerTuple(id, work, vc) = tuple;
                self.report_status(WorkerStatus::TaskRunning(id)).unwrap();

                (work.on_start)();
                (work.work)();
                (work.on_complete)();
                self.report_status(WorkerStatus::Idle).unwrap();

                match vc.send(()) {
                    Ok(()) => {}
                    Err(_e) => {
                        // occurs only if request handle went out of scope
                    }
                }
            }
        }
    }

    fn report_status(&mut self, status: WorkerStatus) -> Result<(), SendError<WorkStatusUpdate>> {
        self.status_send.send(WorkStatusUpdate {
            worker_id: self.id,
            status,
        })
    }
}

struct WorkerTuple(WorkTokenId, WorkToken, Sender<()>);

/// A worker queue is a way of submitting work to a [`WorkerExecutor`](WorkerExecutor).
///
/// A task submitted to the worker will get get put into the worker queue immediately.
/// Dropping a WorkerQueue will force all work handles to be joined.
pub struct WorkerQueue<'exec> {
    executor: &'exec WorkerExecutor,
    handles: Vec<WorkHandle<'exec>>,
}

impl<'exec> Drop for WorkerQueue<'exec> {
    fn drop(&mut self) {
        let handles = self.handles.drain(..);
        for handle in handles {
            let _ = handle.join();
        }
    }
}

impl<'exec> WorkerQueue<'exec> {
    /// Create a new worker queue with a given executor.
    pub fn new(executor: &'exec WorkerExecutor) -> Self {
        Self {
            executor,
            handles: vec![],
        }
    }

    /// Submit some work to do by the queue
    pub fn submit<W: Into<WorkToken>>(&mut self, work: W) -> io::Result<WorkHandle> {
        let handle = self.executor.submit(work)?;
        self.handles.push(handle.clone());
        Ok(handle)
    }

    /// Finishes the WorkerQueue by finishing all submitted tasks.
    pub fn join(mut self) -> io::Result<()> {
        for handle in self.handles.drain(..) {
            handle.join()?;
        }
        Ok(())
    }

    pub fn typed<W: Into<WorkToken>>(self) -> TypedWorkerQueue<'exec, W> {
        TypedWorkerQueue {
            _data: PhantomData,
            queue: self,
        }
    }
}

/// Allows for only submissed of a certain type into the worker queue.
pub struct TypedWorkerQueue<'exec, W: Into<WorkToken>> {
    _data: PhantomData<W>,
    queue: WorkerQueue<'exec>,
}

impl<'exec, W: Into<WorkToken>> TypedWorkerQueue<'exec, W> {
    /// Create a new worker queue with a given executor.
    pub fn new(executor: &'exec WorkerExecutor) -> Self {
        Self {
            _data: PhantomData,
            queue: executor.queue(),
        }
    }

    /// Submit some work to do by the queue
    pub fn submit(&mut self, work: W) -> io::Result<WorkHandle> {
        self.queue.submit(work)
    }

    /// Finishes the WorkerQueue by finishing all submitted tasks.
    pub fn join(mut self) -> io::Result<()> {
        self.queue.join()
    }
}

#[cfg(test)]
mod tests {
    use crate::workqueue::WorkerExecutor;
    use crossbeam::sync::WaitGroup;
    use rand::{thread_rng, Rng};
    use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;
    const WORK_SIZE: usize = 6;
    #[test]
    fn parallelism_works() {
        let mut worker_queue = WorkerExecutor::new(WORK_SIZE).unwrap();

        let wait_group = Arc::new(Barrier::new(WORK_SIZE));
        let mut add_all = Arc::new(AtomicUsize::new(0));

        let mut current_worker = 0;

        for _ in 0..(WORK_SIZE * 2) {
            let add_all = add_all.clone();
            let this_worker = current_worker;
            current_worker += 1;
            worker_queue
                .submit(move || {
                    println!("running worker thread {}", this_worker);
                    add_all.fetch_add(1, Ordering::Relaxed);
                })
                .unwrap();
        }

        worker_queue.finish_jobs().unwrap();
        assert_eq!(add_all.load(Ordering::Acquire), WORK_SIZE * 2);

        for _ in 0..(WORK_SIZE * 2) {
            let add_all = add_all.clone();
            let this_worker = current_worker;
            current_worker += 1;
            worker_queue
                .submit(move || {
                    println!("running worker thread {}", this_worker);
                    add_all.fetch_add(1, Ordering::Relaxed);
                })
                .unwrap();
        }

        worker_queue.join().unwrap();

        assert_eq!(add_all.load(Ordering::Acquire), WORK_SIZE * 4);
    }

    #[test]
    fn worker_queues_provide_protection() {
        let exec = WorkerExecutor::new(WORK_SIZE).unwrap();

        let mut accum = Arc::new(AtomicUsize::new(0));
        {
            let mut queue = exec.queue();
            for i in 0..64 {
                let accum = accum.clone();
                queue
                    .submit(move || {
                        accum.fetch_add(1, Ordering::Relaxed);
                    })
                    .unwrap();
            }

            // queue should drop here
        }

        assert_eq!(accum.load(Ordering::Acquire), 64);
    }

    fn test_executor_pool_size_ensured(pool_size: usize) {
        let mut workers_running = Arc::new(AtomicUsize::new(0));
        let mut max_workers_running = Arc::new(AtomicUsize::new(0));

        let executor = WorkerExecutor::new(pool_size).unwrap();
        {
            let mut queue = executor.queue();
            for _ in 0..4 * pool_size {
                let workers_running = workers_running.clone();
                let max_workers_running = max_workers_running.clone();
                queue.submit(move || {
                    workers_running.fetch_add(1, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(100));
                    workers_running.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |running| {
                        max_workers_running.fetch_update(
                            Ordering::SeqCst,
                            Ordering::SeqCst,
                            |max| {
                                if running > max {
                                    Some(running)
                                } else {
                                    None
                                }
                            },
                        );
                        None
                    });

                    workers_running.fetch_sub(1, Ordering::SeqCst);
                });
            }

            queue.join().expect("worker task failed :(");
        }

        let max_workers_running = max_workers_running.load(Ordering::Acquire);
        println!("max running workers: {}", max_workers_running);
        assert!(max_workers_running <= pool_size);
    }

    #[test]
    fn only_correct_number_of_workers_run() {
        test_executor_pool_size_ensured(1);
        test_executor_pool_size_ensured(2);
        test_executor_pool_size_ensured(4);
        test_executor_pool_size_ensured(8);
    }

    #[test]
    fn can_stop_after_panic() {
        let executor = WorkerExecutor::new(1).unwrap();
        let job = executor.submit(|| panic!("WOOH I PANICKED")).unwrap();
        job.join()
            .expect_err("Should expect an error because a panic occurred");
        println!("any panicked = {}", executor.any_panicked());
        assert!(executor.any_panicked());
    }
}
