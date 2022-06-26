//! Standard operations used by freight

use std::num::NonZeroUsize;
use std::io;
use assemble_core::Executable;
use assemble_core::work_queue::WorkerExecutor;
use petgraph::graph::{DiGraph, NodeIndex};
use assemble_core::task::{TaskId, TaskOrdering, TaskOrderingKind};
use std::collections::{HashMap, HashSet};
use petgraph::Outgoing;
use petgraph::algo::tarjan_scc;
use petgraph::prelude::EdgeRef;
use crate::core::{ConstructionError, ExecutionGraph, ExecutionPlan, Type};

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
pub fn try_creating_plan<E : Executable>(mut exec_g: ExecutionGraph<E>) -> Result<ExecutionPlan<E>, ConstructionError> {

    let idx_to_old_graph_idx = exec_g.graph.node_indices()
        .map(|idx| (idx, exec_g.graph[idx].task_id().clone()))
        .collect::<HashMap<_, _>>();

    let critical_path = {
        let mut critical_path: HashSet<TaskId> = HashSet::new();

        let mut task_stack = exec_g.requested_tasks.clone();

        while let Some(task_id) = task_stack.pop() {
            if critical_path.contains(&task_id) {
                continue;
            } else {
                critical_path.insert(task_id.clone());
            }

            let id = find_node(&exec_g.graph, &task_id).ok_or(ConstructionError::IdentifierNotFound(task_id))?;
            for outgoing in exec_g.graph.edges_directed(id, Outgoing) {
                let target = outgoing.target();

                match outgoing.weight() {
                    TaskOrderingKind::DependsOn | TaskOrderingKind::FinalizedBy => {
                        let identifier = exec_g.graph[target].task_id().clone();
                        if !critical_path.contains(&identifier) {
                            task_stack.push(identifier);
                        }
                    }
                    _ => {
                        continue
                    }
                }
            }
        }

        critical_path
    };


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

    for edge in edges {
        let from = &idx_to_old_graph_idx[&edge.source()];
        let to = &idx_to_old_graph_idx[&edge.target()];
        let (from, ty, to) = match edge.weight {
            TaskOrderingKind::RunsBefore => {
                (to, Type::RunAfter,from)
            }
            TaskOrderingKind::FinalizedBy => {
                (to, Type::Finalizer, from)
            }
            TaskOrderingKind::RunsAfter |
            TaskOrderingKind::DependsOn => {
                (from, Type::RunAfter, to)
            }
        };
        let from_idx = id_to_new_graph_idx[from];
        let to_idx = id_to_new_graph_idx[to];
        new_graph.add_edge(
            from_idx, to_idx, ty
        );
    }

    let scc_s = tarjan_scc(&new_graph);

    if scc_s.len() != new_graph.node_count()  {
        // Since we know the number of scc's is N - 1, where N is the number of nodes, and that each node within the graph
        // appears exactly once in all of the sccs, that means theres N nodes among N  - 1 buckets. As such, ther must be
        // at least one bucket with more than one node within it.

        let cycle = scc_s.into_iter().find(|comp| comp.len() > 1)
            .expect("pigeonhole theory prevents this")
            .into_iter()
            .map(|idx: NodeIndex| new_graph[idx].task_id().clone())
            .collect();

        return Err(ConstructionError::CycleFound { cycle });
    }

    Ok(ExecutionPlan::new(new_graph, exec_g.requested_tasks))

}

fn find_node<E : Executable, W>(graph: &DiGraph<E, W>, id: &TaskId) -> Option<NodeIndex> {
    graph.node_indices()
        .find(|idx| graph[*idx].task_id() == id)

}
