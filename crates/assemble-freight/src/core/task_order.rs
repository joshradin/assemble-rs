use assemble_core::task::TaskIdentifier;
use assemble_core::ExecutableTask;
use petgraph::graph::DiGraph;
use petgraph::stable_graph::StableDiGraph;
use std::collections::{HashMap, VecDeque};
use petgraph::Direction;
use petgraph::visit::EdgeRef;

/*

How do we construct the ExecutionPlan for an ExecutionGraph?

Make it so any task that doesn't have any incoming nodes are available tasks to be processed.
Once a task is completed, remove the corresponding node from the plan. After each task is complete,
a search should be done for tasks that are available to be done.

How do we determine if a task should be in ExecutionPlan? A main line of tasks should be constructed
using the requested tasks. Only tasks that are depended upon or finalized by these tasks should be
included in the final plan

 */

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Type {
    RunAfter,
    Finalizer,
}

/// An execution plan is guaranteed to have no cycles, and each task is run in the correct order.
/// The execution graph can only be created from an [`ExecutionGraph`](crate::core::ExecutionGraph)
pub struct ExecutionPlan<E: ExecutableTask> {
    graph: DiGraph<TaskIdentifier, Type>,
    id_to_task: HashMap<TaskIdentifier, E>,
    task_queue: VecDeque<TaskIdentifier>,
}

impl<E: ExecutableTask> ExecutionPlan<E> {

    pub fn new(mut graph: DiGraph<E, Type>) -> Self {

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
        Self {
            graph: fixed,
            id_to_task,
            task_queue: Default::default()
        }
    }

    /// Get the next task that can be run.
    pub fn pop_task(&mut self) -> Option<E> {
        self.task_queue
            .pop_front()
            .and_then(|id| self.id_to_task.remove(&id))
    }

    /// Report to the execution plan that the given task has completed.
    ///
    /// If the task has completed successfully, then the node is removed along with all connected edges.
    /// Otherwise, only the edges that are to finalizer tasks are removed.
    pub fn report_task_status(&mut self, id: &TaskIdentifier, success: bool) {
        let index = self.graph.node_indices().find(|idx| self.graph.node_weight(*idx).unwrap() == id)
            .expect(&format!("{} not in graph", id));
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
        self.task_queue.extend(available);
    }
}
