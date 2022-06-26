use std::cmp::{Ordering, Reverse};
use assemble_core::task::TaskOrdering;
use assemble_core::Executable;
use petgraph::graph::{DefaultIx, DiGraph};
use petgraph::stable_graph::StableDiGraph;
use std::collections::{BinaryHeap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt::Debug;
use std::process::id;
use petgraph::algo::{connected_components, tarjan_scc, toposort};
use petgraph::prelude::*;
use assemble_core::identifier::TaskId;
use crate::core::{ConstructionError, ExecutionGraph};

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
pub struct ExecutionPlan<E: Executable> {
    graph: DiGraph<TaskId, Type>,
    id_to_task: HashMap<TaskId, E>,
    task_queue: BinaryHeap<Reverse<WorkRequest>>,
    task_requests: Vec<TaskId>,
    waiting_on: HashSet<TaskId>
}

impl<E: Executable> ExecutionPlan<E> {

    pub fn new(mut graph: DiGraph<E, Type>, requests: Vec<TaskId>) -> Self {
        let fixed = graph.map(
            |idx, node| node.task_id().clone(),
            |idx, edge| *edge
        );
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
        plan.discover_available_tasks();
        plan
    }

    /// Get whether there are tasks available to be picked up or eventually
    pub fn finished(&self) -> bool {
        self.task_queue.is_empty() && self.waiting_on.is_empty()
    }

    /// Get the next task that can be run.
    pub fn pop_task(&mut self) -> Option<E> {
        let out = self.task_queue
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
        let index = self.graph.node_indices().find(|idx| self.graph.node_weight(*idx).unwrap() == id)
            .expect(&format!("{} not in graph", id));
        self.waiting_on.remove(id);
        if success {
            self.graph.remove_node(index);
        } else {
            let outgoing = self.graph.edges_directed(index, Direction::Outgoing)
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
            let incoming = self.graph.edges_directed(index, Direction::Incoming);
            if incoming.count() == 0 {
                let task = self.graph.node_weight(index).unwrap();
                available.push(task.clone());
            }
        }
        let with_prio = available.into_iter()
            .map(|id| {
                let prio = match self.task_requests.iter().position(|p| p == &id) {
                    None => {
                        Priority::OnPath
                    }
                    Some(pos) => {
                        Priority::Requested(pos)
                    }
                };
                Reverse(WorkRequest {
                    identifier: id,
                    priority: prio
                }
                )
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
    OnPath
}

#[derive(Debug)]
struct WorkRequest {
    identifier: TaskId,
    priority: Priority
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
mod test {


}