use crate::core::{ConstructionError, ExecutionGraph};
use array2d::Array2D;
use assemble_core::identifier::TaskId;
use assemble_core::task::{ExecutableTask, FullTask, TaskOrdering};
use colored::Colorize;
use petgraph::algo::{connected_components, tarjan_scc, toposort};
use petgraph::graph::{DefaultIx, DiGraph};
use petgraph::prelude::*;
use petgraph::stable_graph::StableDiGraph;
use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};
use std::fmt::Debug;
use std::process::id;

/*

How do we construct the ExecutionPlan for an ExecutionGraph?

Make it so any task that doesn't have any incoming nodes are available tasks to be processed.
Once a task is completed, remove the corresponding node from the plan. After each task is complete,
a search should be done for tasks that are available to be done.

How do we determine if a task should be in ExecutionPlan? A main line of tasks should be constructed
using the requested tasks. Only tasks that are depended upon or finalized by these tasks should be
included in the final plan

 */

/// Type of ordering
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Type {
    RunAfter,
    Finalizer,
}

/// An execution plan is guaranteed to have no cycles, and each task is run in the correct order.
/// The execution graph can only be created from an [`ExecutionGraph`](crate::core::ExecutionGraph)
#[derive(Debug)]
pub struct ExecutionPlan {
    graph: DiGraph<TaskId, Type>,
    id_to_task: HashMap<TaskId, Box<dyn FullTask>>,
    task_queue: BinaryHeap<Reverse<WorkRequest>>,
    task_requests: Vec<TaskId>,
    waiting_on: HashSet<TaskId>,
}

impl ExecutionPlan {
    pub fn new(mut graph: DiGraph<Box<dyn FullTask>, Type>, requests: Vec<TaskId>) -> Self {
        let fixed = graph.map(|idx, node| node.task_id().clone(), |idx, edge| *edge);
        let mut id_to_task = HashMap::new();
        let (nodes, _) = graph.into_nodes_edges();
        for node in nodes {
            let task = node.weight;
            let id = task.task_id().clone();
            id_to_task.insert(id, task);
        }
        let mut plan = Self {
            graph: fixed,
            id_to_task,
            task_queue: Default::default(),
            task_requests: requests,
            waiting_on: Default::default(),
        };
        plan.remove_redundant_edges();
        plan.discover_available_tasks();
        plan
    }

    /// Removes redundant edges from a graph. Should only really do anything once. Redundant edges
    /// are defined as edges such given three points A, B, C. if A depends on C and B, and B depends on C,
    /// then the edge from A to C is redundant because it's already covered by the transitive property.
    pub fn remove_redundant_edges(&mut self) {
        let graph = &mut self.graph;
        let n = graph.node_count();
        let mut reflexive_reduction = Array2D::filled_with(false, n, n);

        for edge in graph.edge_references() {
            let from = edge.source().index();
            let target = edge.target().index();

            reflexive_reduction[(from, target)] = true;
        }

        for i in 0..n {
            reflexive_reduction[(i, i)] = false;
        }

        let mut edges_to_remove = Vec::new();

        for j in 0..n {
            for i in 0..n {
                if reflexive_reduction[(i, j)] {
                    for k in 0..n {
                        if reflexive_reduction[(j, k)] {
                            reflexive_reduction[(i, k)] = false;
                            edges_to_remove.push((i, k));
                        }
                    }
                }
            }
        }

        let mut count = 0;
        for (source, target) in edges_to_remove {
            let source = NodeIndex::new(source);
            let target = NodeIndex::new(target);

            let find_edge = graph.find_edge(source, target).unwrap();
            graph.remove_edge(find_edge);
            count += 1;
        }

        info!(
            "removed {} redundant edges from execution plan",
            count.to_string().bold()
        );
    }

    /// Get whether there are tasks available to be picked up or eventually
    pub fn finished(&self) -> bool {
        self.task_queue.is_empty() && self.waiting_on.is_empty()
    }

    /// Get the next task that can be run.
    pub fn pop_task(&mut self) -> Option<Box<dyn FullTask>> {
        let out = self
            .task_queue
            .pop()
            .map(|req| req.0.identifier)
            .and_then(|id| self.id_to_task.remove(&id));
        if let Some(out) = &out {
            let id = out.task_id().clone();
            self.waiting_on.insert(id);
        }
        out
    }

    /// Report to the execution plan that the given task has completed.
    ///
    /// If the task has completed successfully, then the node is removed along with all connected edges.
    /// Otherwise, only the edges that are to finalizer tasks are removed.
    pub fn report_task_status(&mut self, id: &TaskId, success: bool) {
        let index = self
            .graph
            .node_indices()
            .find(|idx| self.graph.node_weight(*idx).unwrap() == id)
            .expect(&format!("{} not in graph", id));
        self.waiting_on.remove(id);
        if success {
            self.graph.remove_node(index);
        } else {
            let outgoing = self
                .graph
                .edges_directed(index, Direction::Outgoing)
                .filter(|edge| *edge.weight() == Type::Finalizer)
                .map(|edge| edge.id())
                .collect::<Vec<_>>();
            for edge in outgoing {
                self.graph.remove_edge(edge).unwrap();
            }
        }
        self.discover_available_tasks();
    }

    fn discover_available_tasks(&mut self) {
        let mut available = vec![];
        for index in self.graph.node_indices() {
            let incoming = self.graph.edges_directed(index, Direction::Outgoing);
            if incoming.count() == 0 {
                let task = self.graph.node_weight(index).unwrap();
                available.push(task.clone());
            }
        }
        let with_prio = available.into_iter().map(|id| {
            let prio = match self.task_requests.iter().position(|p| p == &id) {
                None => Priority::OnPath,
                Some(pos) => Priority::Requested(pos),
            };
            Reverse(WorkRequest {
                identifier: id,
                priority: prio,
            })
        });
        self.task_queue.extend(with_prio);
    }
}

/// The priority of a task
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Priority {
    /// The task was requested on the command line
    Requested(#[doc("The arg number of the task in the command line")] usize),
    /// The task appeared on the critical path
    OnPath,
}

#[derive(Debug)]
struct WorkRequest {
    identifier: TaskId,
    priority: Priority,
}

impl Eq for WorkRequest {}

impl PartialEq<Self> for WorkRequest {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl PartialOrd<Self> for WorkRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for WorkRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

#[cfg(test)]
mod test {}
