use assemble_core::identifier::TaskId;
use assemble_core::project::requests::TaskRequests;
use assemble_core::task::flags::WeakOptionsDecoder;
use assemble_core::task::FullTask;
use colored::Colorize;
use log::Level;

use petgraph::data::DataMap;
use petgraph::graph::DiGraph;
use petgraph::prelude::*;

use ptree::PrintConfig;
use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fmt;
use std::fmt::Debug;

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
    task_requests: TaskRequests,
    waiting_on: HashSet<TaskId>,
}

impl ExecutionPlan {
    pub fn new(graph: DiGraph<Box<dyn FullTask>, Type>, requests: TaskRequests) -> Self {
        let fixed = graph.map(|_idx, node| node.task_id().clone(), |_idx, edge| *edge);
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

    /// Current number of tasks present in execution plan
    pub fn len(&self) -> usize {
        self.graph.node_count()
    }

    /// Removes redundant edges from a graph. Should only really do anything once. Redundant edges
    /// are defined as edges such given three points A, B, C. if A depends on C and B, and B depends on C,
    /// then the edge from A to C is redundant because it's already covered by the transitive property.
    pub fn remove_redundant_edges(&mut self) {
        let graph = &mut self.graph;
        let nodes = graph.node_indices();

        let mut count = 0;
        for i in nodes.clone() {
            for j in nodes.clone() {
                for k in nodes.clone() {
                    if let Some(edge) = graph.find_edge(i, k) {
                        if graph.contains_edge(i, j) && graph.contains_edge(j, k) {
                            graph.remove_edge(edge);
                            count += 1;
                        }
                    }
                }
            }
        }

        debug!(
            "removed {} redundant edges from execution plan",
            count.to_string().bold()
        );
    }

    /// Check whether the execution plan actually has anything to do
    pub fn is_empty(&self) -> bool {
        self.task_requests.requested_tasks().is_empty()
    }

    /// Get whether there are tasks available to be picked up or eventually
    pub fn finished(&self) -> bool {
        self.task_queue.is_empty() && self.waiting_on.is_empty()
    }

    /// Get the next task that can be run.
    pub fn pop_task(&mut self) -> Option<(Box<dyn FullTask>, Option<WeakOptionsDecoder>)> {
        let out = self
            .task_queue
            .pop()
            .map(|req| req.0.identifier)
            .and_then(|id| self.id_to_task.remove(&id));
        if let Some(out) = out {
            let id = out.task_id().clone();
            self.waiting_on.insert(id.clone());
            if let Some(weak) = self.task_requests.decoder(&id) {
                Some((out, Some(weak)))
            } else {
                Some((out, None))
            }
        } else {
            None
        }
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
            .unwrap_or_else(|| panic!("{} not in graph", id));
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
            let prio = match self
                .task_requests
                .requested_tasks()
                .iter()
                .position(|p| p == &id)
            {
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

    pub fn print_plan(&self, level: Level) {
        let mut buffer = Vec::new();
        let mut print_config = PrintConfig::default();

        print_config.characters.down = "|".to_string();
        print_config.characters.down_and_right = "+".to_string();
        print_config.characters.turn_right = "\\".to_string();
        print_config.characters.right = "-".to_string();

        let first = self.task_requests.requested_tasks().first().unwrap();
        let node_index = &self
            .graph
            .node_indices()
            .find(|i| &self.graph[*i] == first)
            .unwrap();
        ptree::graph::write_graph_with(&self.graph, *node_index, &mut buffer, &print_config)
            .map_err(|_e| fmt::Error)
            .expect("couldn't write graph");
        let string = String::from_utf8(buffer).expect("not utf-8");
        for line in string.lines() {
            log!(level, "{line}");
        }
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
