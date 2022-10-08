//! Standard operations used by freight

use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::identity;
use std::num::NonZeroUsize;
use std::thread::sleep;
use std::time::{Duration, Instant};
use std::{io, panic};

use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use itertools::Itertools;
use log::Level;
use petgraph::algo::tarjan_scc;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::prelude::EdgeRef;
use petgraph::Outgoing;
use rayon::iter::{ParallelBridge, ParallelIterator};

use assemble_core::identifier::TaskId;
use assemble_core::logging::{ConsoleMode, LOGGING_CONTROL};
use assemble_core::prelude::ProjectError;
use assemble_core::project::requests::TaskRequests;
use assemble_core::project::SharedProject;
use assemble_core::task::task_container::FindTask;
use assemble_core::task::task_executor::TaskExecutor;
use assemble_core::task::{
    force_rerun, ExecutableTask, FullTask, HasTaskId, TaskOrdering, TaskOrderingKind, TaskOutcome,
};
use assemble_core::utilities::measure_time;
use assemble_core::work_queue::WorkerExecutor;

use crate::cli::{main_progress_bar_style, FreightArgs};
use crate::core::{ConstructionError, ExecutionGraph, ExecutionPlan, Type};
use crate::{FreightError, FreightResult, TaskResolver, TaskResult, TaskResultBuilder};
use assemble_core::error::PayloadError;
use assemble_core::file_collection::Component::Path;

/// Initialize the task executor.
pub fn init_executor(num_workers: NonZeroUsize) -> io::Result<WorkerExecutor> {
    let num_workers = num_workers.get();

    WorkerExecutor::new(num_workers)
}

/// Try creating an execution plan from an execution graph. Will fail if it's not possible to create
/// a plan because there's a cycle within tasks or it's impossible to create a fully linear execution
/// of the given tasks.
///
/// # Error Examples
///
/// ## Task Cycles
/// Task cycles occur when 2 or more tasks dependencies are circular to each other. For example
/// ```text
///  task 1 depends on task 2
///  task 2 depends on task 3
///  task 3 depends on task 1
/// ```
/// It's not possible to run these tasks such that one of the task's dependencies are are already
/// completed.
///
/// ## Linear Execution Impossible
/// If you were to run the tasks one after the each other, it would be impossible to put the tasks
/// in some order where dependencies between the tasks are maintained. For example
/// ```text
///  task 1 runs before task 2
///  task 1 runs after task 3
///  task 2 runs before task 3
/// ```
/// can not be represented in a linear timeline, as task 1 must run before task 2 and after task 3,
/// while task 3 must run after task 2.
/// > This is still a task cycle, but it's not as obvious since it relies on the before/after operations
/// > instead of direct edges.
///
#[cold]
pub fn try_creating_plan(exec_g: ExecutionGraph) -> Result<ExecutionPlan, ConstructionError> {
    trace!("creating plan from {:#?}", exec_g);

    let idx_to_old_graph_idx = exec_g
        .graph
        .node_indices()
        .map(|idx| (idx, exec_g.graph[idx].task_id().clone()))
        .collect::<HashMap<_, _>>();

    let critical_path = {
        let mut critical_path: HashSet<TaskId> = HashSet::new();

        let mut task_stack: VecDeque<_> = exec_g
            .requested_tasks
            .requested_tasks()
            .iter()
            .cloned()
            .collect();

        while let Some(task_id) = task_stack.pop_front() {
            if critical_path.contains(&task_id) {
                continue;
            } else {
                critical_path.insert(task_id.clone());
            }

            let id = find_node(&exec_g.graph, &task_id)
                .ok_or(ConstructionError::IdentifierNotFound(task_id))?;
            for outgoing in exec_g.graph.edges_directed(id, Outgoing) {
                let target = outgoing.target();

                match outgoing.weight() {
                    TaskOrderingKind::DependsOn | TaskOrderingKind::FinalizedBy => {
                        let identifier = exec_g.graph[target].task_id().clone();
                        if !critical_path.contains(&identifier) {
                            task_stack.push_back(identifier);
                        }
                    }
                    _ => continue,
                }
            }
        }

        critical_path
    };

    debug!(
        "critical path: {{{}}}",
        critical_path
            .iter()
            .map(|id: &TaskId| id.to_string())
            .join(", ")
    );
    debug!("The critical path are the tasks that are requested and all of their dependencies");

    let mut new_graph = DiGraph::new();
    let (nodes, edges) = exec_g.graph.into_nodes_edges();

    let mut id_to_new_graph_idx = HashMap::new();

    for node in nodes {
        let task = node.weight;
        let task_id = task.task_id().clone();
        if critical_path.contains(&task_id) {
            let idx = new_graph.add_node(task);
            id_to_new_graph_idx.insert(task_id, idx);
        }
    }

    trace!("new graph (nodes only): {:#?}", new_graph);

    for edge in edges {
        let from = &idx_to_old_graph_idx[&edge.source()];
        let to = &idx_to_old_graph_idx[&edge.target()];
        let (from, ty, to) = match edge.weight {
            TaskOrderingKind::RunsBefore => (to, Type::RunAfter, from),
            TaskOrderingKind::FinalizedBy => (to, Type::Finalizer, from),
            TaskOrderingKind::RunsAfter | TaskOrderingKind::DependsOn => (from, Type::RunAfter, to),
        };
        let from_idx = id_to_new_graph_idx[from];
        let to_idx = id_to_new_graph_idx[to];
        new_graph.add_edge(from_idx, to_idx, ty);
    }

    let scc_s = tarjan_scc(&new_graph);

    if scc_s.len() != new_graph.node_count() {
        // Since we know the number of scc's is N - 1, where N is the number of nodes, and that each node within the graph
        // appears exactly once in all of the sccs, that means theres N nodes among N  - 1 buckets. As such, ther must be
        // at least one bucket with more than one node within it.

        let cycle = scc_s
            .into_iter()
            .find(|comp| comp.len() > 1)
            .expect("pigeonhole theory prevents this")
            .into_iter()
            .map(|idx: NodeIndex| new_graph[idx].task_id().clone())
            .collect();

        return Err(ConstructionError::CycleFound { cycle });
    }

    Ok(ExecutionPlan::new(new_graph, exec_g.requested_tasks))
}

fn find_node<W>(graph: &DiGraph<Box<dyn FullTask>, W>, id: &TaskId) -> Option<NodeIndex> {
    graph.node_indices().find(|idx| graph[*idx].task_id() == id)
}

/// The main entry point into freight.
pub fn execute_tasks(
    project: &SharedProject,
    args: &FreightArgs,
) -> FreightResult<Vec<TaskResult>> {
    let start_instant = Instant::now();
    let handle = args.logging().init_root_logger().ok().flatten();

    if args.rerun_tasks() {
        force_rerun(true);
    }

    let exec_graph = {
        let resolver = TaskResolver::new(project);
        let task_requests = args.task_requests(project)?;
        resolver.to_execution_graph(task_requests)?
    };

    trace!("created exec graph: {:#?}", exec_graph);
    let mut exec_plan = try_creating_plan(exec_graph)?;
    exec_plan.print_plan(Level::Trace);

    if exec_plan.is_empty() {
        return Ok(vec![]);
    }

    debug!(
        "{project} plan creation time: {:.3} sec",
        start_instant.elapsed().as_secs_f32()
    );

    let executor = init_executor(NonZeroUsize::new(args.workers()).unwrap())?;

    let mut results = vec![];

    let mut work_queue = TaskExecutor::new(project.clone(), &executor);

    let mut progress = MultiProgress::with_draw_target(ProgressDrawTarget::stdout_with_hz(u8::MAX));

    let mut worker_bars = vec![];
    let mut available_workers = VecDeque::from_iter(0..args.workers());
    let mut in_use_workers: HashMap<TaskId, usize> = HashMap::new();

    let bar = ProgressBar::new(exec_plan.len() as u64)
        .with_style(main_progress_bar_style(false))
        .with_message("Executing");

    bar.enable_steady_tick(Duration::from_millis(100));

    let main_bar = progress.add(bar);
    main_bar.tick();

    for _ in 0..args.workers() {
        let bar = progress.add(
            ProgressBar::new_spinner().with_style(ProgressStyle::with_template("> {msg}").unwrap()),
        );
        worker_bars.push(bar.clone());
    }

    progress.set_move_cursor(false);

    if let ConsoleMode::Rich = args.logging().console.resolve() {
        LOGGING_CONTROL.start_progress_bar(&progress).unwrap();
    }

    let mut results_builders = HashMap::new();

    let task_execution_start_time = Instant::now();

    while !(exec_plan.finished() || executor.any_panicked()) {
        if let Some(worker_index) = available_workers.pop_front() {
            if let Some((mut task, decs)) = exec_plan.pop_task() {
                trace!("loading task {} into task queue", task.task_id());
                let result_builder = TaskResultBuilder::new(task.task_id().clone());
                results_builders.insert(task.task_id().clone(), result_builder);

                let task_bar = { worker_bars[worker_index].clone() };

                if let Some(weak_decoder) = decs {
                    let task_options = task.options_declarations().unwrap();
                    let upgraded_decoder = weak_decoder.upgrade(&task_options)?;
                    task.try_set_from_decoder(&upgraded_decoder)?;
                }

                task_bar.set_message(format!("{}", task.task_id()));
                task_bar.tick();
                in_use_workers.insert(task.task_id().clone(), worker_index);
                work_queue.queue_task(task)?;
            } else {
                available_workers.push_front(worker_index);
            }
        }
        // sleep(Duration::from_millis(100));
        for (task_id, output) in work_queue.finished_tasks() {
            trace!("received task {} from task queue", task_id);
            let outcome: Option<TaskOutcome> = if let &Ok((up_to_date, did_work)) = &output {
                match (up_to_date, did_work) {
                    (true, did_work) => {
                        if log::log_enabled!(Level::Debug) {
                            info!(
                                "{} - {}",
                                format!("> Task {}", task_id).bold(),
                                "UP-TO-DATE".italic().yellow()
                            );
                        }
                        Some(if did_work {
                            TaskOutcome::UpToDate
                        } else {
                            TaskOutcome::NoSource
                        })
                    }
                    (false, true) => Some(TaskOutcome::Executed),
                    (false, false) => {
                        if log::log_enabled!(Level::Debug) {
                            info!(
                                "{} - {}",
                                format!("> Task {}", task_id).bold(),
                                "SKIPPED".italic().yellow()
                            );
                        }
                        Some(TaskOutcome::Skipped)
                    }
                }
            } else {
                None
            };

            let task_bar_index = in_use_workers[&task_id];
            let task_bar = &worker_bars[task_bar_index];
            available_workers.push_front(task_bar_index);

            task_bar.set_message("");
            task_bar.tick();

            if !output.is_ok() {
                error!("Task {} FAILED", task_id);
                warn!("  > {}", output.as_ref().unwrap_err());
                main_bar.set_style(main_progress_bar_style(true));
            }

            main_bar.inc(1);

            exec_plan.report_task_status(&task_id, output.is_ok());
            let result_builder = results_builders.remove(&task_id).unwrap();
            let work_result =
                result_builder.finish(output.map(|_| outcome.expect("should be set")));
            results.push(work_result);
        }
    }

    trace!("received task completion notice.");
    // info!("");
    // info!(
    //     "finished executing tasks in {:.3} sec",
    //     task_execution_start_time.elapsed().as_secs_f32()
    // );

    let (finished_results, error) = work_queue.finish();
    for (task_id, output) in finished_results {
        let outcome: Option<TaskOutcome> = if let &Ok((up_to_date, did_work)) = &output {
            match (up_to_date, did_work) {
                (true, did_work) => {
                    if log::log_enabled!(Level::Debug) {
                        info!(
                            "{} - {}",
                            format!("> Task {}", task_id).bold(),
                            "UP-TO-DATE".italic().yellow()
                        );
                    }
                    Some(if did_work {
                        TaskOutcome::UpToDate
                    } else {
                        TaskOutcome::NoSource
                    })
                }
                (false, true) => Some(TaskOutcome::Executed),
                (false, false) => {
                    if log::log_enabled!(Level::Debug) {
                        info!(
                            "{} - {}",
                            format!("> Task {}", task_id).bold(),
                            "SKIPPED".italic().yellow()
                        );
                    }
                    Some(TaskOutcome::Skipped)
                }
            }
        } else {
            None
        };

        let task_bar_index = in_use_workers[&task_id];
        let task_bar = &worker_bars[task_bar_index];
        available_workers.push_front(task_bar_index);

        task_bar.set_message("");
        task_bar.tick();

        if !output.is_ok() {
            error!("Task {} FAILED", task_id);
            main_bar.set_style(main_progress_bar_style(true));
        }

        main_bar.inc(1);

        exec_plan.report_task_status(&task_id, output.is_ok());
        let result_builder = results_builders.remove(&task_id).unwrap();
        let work_result = result_builder.finish(output.map(|_| outcome.unwrap()));
        results.push(work_result);
    }

    let panicked = matches!(&error, Some(_));

    trace!(
        "freight task completion time: {:.3} sec",
        start_instant.elapsed().as_secs_f32()
    );

    measure_time("finish and clear bars", Level::Trace, || {
        worker_bars
            .into_iter()
            .par_bridge()
            .for_each(|bar| bar.finish_and_clear());
    });

    if !panicked {
        measure_time("join executor", Level::Trace, || {
            executor.join() // force the executor to terminate safely.
        })?;
    } else {
        measure_time("join executor", Level::Trace, || {
            identity(executor).finish_jobs();
        });
        error!("A panic occurred within a task. Can't return good results");
        panic::resume_unwind(error.unwrap());
    }

    if let ConsoleMode::Rich = args.logging().console {
        LOGGING_CONTROL.end_progress_bar();
    }

    trace!(
        "freight execution time: {:.3} sec",
        start_instant.elapsed().as_secs_f32()
    );

    if let Some(handle) = handle {
        LOGGING_CONTROL.stop_logging();
        handle.join().unwrap();
    }

    Ok(results)
}
