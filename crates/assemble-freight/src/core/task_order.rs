use assemble_core::task::{TaskIdentifier, TaskOrdering};
use assemble_core::ExecutableTask;
use petgraph::graph::{DefaultIx, DiGraph};
use petgraph::stable_graph::StableDiGraph;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::process::id;
use petgraph::algo::{connected_components, tarjan_scc, toposort};
use petgraph::prelude::*;
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
        let mut plan = Self {
            graph: fixed,
            id_to_task,
            task_queue: Default::default()
        };
        plan.discover_available_tasks();
        plan
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
/// > This is actually the same as a task cycle, but it's easier to visualize as a linear construct.
///
#[cold]
pub fn try_creating_plan<E : ExecutableTask>(mut exec_g: ExecutionGraph<E>) -> Result<ExecutionPlan<E>, ConstructionError> {

    let idx_to_old_graph_idx = exec_g.graph.node_indices()
        .map(|idx| (idx, exec_g.graph[idx].task_id().clone()))
        .collect::<HashMap<_, _>>();

    let critical_path = {
        let mut critical_path: HashSet<TaskIdentifier> = HashSet::new();

        let mut task_stack = Vec::from_iter(exec_g.requested_tasks);

        while let Some(task_id) = task_stack.pop() {
            if critical_path.contains(&task_id) {
                continue;
            }

            let id = find_node(&exec_g.graph, &task_id).ok_or(ConstructionError::IdentifierNotFound(task_id))?;
            for outgoing in exec_g.graph.edges_directed(id, Outgoing) {
                let target = outgoing.target();

                match outgoing.weight() {
                    TaskOrdering::DependsOn(_) | TaskOrdering::FinalizedBy(_) => {
                        let identifier = exec_g.graph[target].task_id().clone();
                        if !critical_path.contains(&identifier) {
                            critical_path.insert(identifier.clone());
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
            TaskOrdering::RunsBefore(_) => {
                (to, Type::RunAfter,from)
            }
            TaskOrdering::FinalizedBy(_) => {
                (to, Type::Finalizer, from)
            }
            TaskOrdering::RunsAfter(_) |
            TaskOrdering::DependsOn(_) => {
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

    Ok(ExecutionPlan::new(new_graph))

}

fn find_node<E : ExecutableTask, W>(graph: &DiGraph<E, W>, id: &TaskIdentifier) -> Option<NodeIndex> {
    graph.node_indices()
        .find(|idx| graph[*idx].task_id() == id)

}