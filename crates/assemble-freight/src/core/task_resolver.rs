use assemble_core::project::ProjectError;
use assemble_core::task::task_container::TaskContainer;
use assemble_core::task::{TaskIdentifier, TaskOrdering};
use assemble_core::{ExecutableTask, Project};
use petgraph::prelude::*;
use petgraph::visit::Visitable;
use std::collections::{HashMap, HashSet, VecDeque};
use crate::core::ConstructionError;

pub struct TaskResolver<'proj, T: ExecutableTask> {
    project: &'proj mut Project<T>,
}

impl<'proj, T: ExecutableTask> TaskResolver<'proj, T> {
    pub fn new(project: &'proj mut Project<T>) -> Self {
        Self { project }
    }

    /// Try to find an identifier corresponding to the given id.
    ///
    /// Right now, only exact matches are allowed.
    pub fn try_find_identifier(&self, id: &str) -> Option<TaskIdentifier> {
        self.project
            .registered_tasks()
            .into_iter()
            .find(|task| task == &id)
    }

    /// Create a task resolver using the given set of tasks as a starting point. Not all tasks
    /// registered to the project will be added to the tasks,
    /// just the ones that are required for the specified tasks to be ran.
    ///
    /// # Error
    /// Will return Err() if any of the [`ExecutionGraph`](ExecutionGraph) rules are invalidated.
    ///
    /// # Example
    /// ```rust
    /// # use assemble_core::Project;
    /// use assemble_core::task::Empty;
    /// let mut project = Project::default();
    /// project.task::<Empty>("task1").unwrap();
    /// project.task::<Empty>("task2").unwrap().configure(|_, opts, _| opts.depend_on("task1"))
    /// ```
    pub fn to_execution_graph<I: IntoIterator<Item = &'proj TaskIdentifier>>(
        mut self,
        tasks: I,
    ) -> Result<ExecutionGraph<T>, ConstructionError> {
        let mut task_id_graph = TaskIdentifierGraph::new();

        let mut task_container = self.project.take_task_container();

        let mut task_queue: VecDeque<TaskIdentifier> = VecDeque::new();
        let requested = tasks.into_iter().cloned().collect::<Vec<_>>();
        task_queue.extend(requested.clone());

        let mut visited = HashSet::new();

        while let Some(task_id) = task_queue.pop_front() {
            if visited.contains(&task_id) {
                continue
            }

            task_id_graph.add_id(task_id.clone());
            visited.insert(task_id.clone());

            let config_info = task_container.configure_task(task_id.clone(), self.project)?;
            for ordering in config_info.ordering {
                let next_id = match &ordering {
                    TaskOrdering::DependsOn(i) => i,
                    TaskOrdering::FinalizedBy(i) => i,
                    TaskOrdering::RunsAfter(i) => i,
                    TaskOrdering::RunsBefore(i) => i,
                };
                if !task_id_graph.contains_id(next_id) {
                    task_id_graph.add_id(next_id.clone());
                }
                task_queue.push_back(next_id.clone());
                task_id_graph.add_task_ordering(task_id.clone(), next_id.clone(), ordering);
            }
        }

        let execution_graph = task_id_graph.map_with(&mut task_container, &self.project)?;
        Ok(ExecutionGraph { graph: execution_graph, requested_tasks: requested })
    }
}
/// The Execution Plan provides a plan of executable tasks that
/// the task executor can execute.
///
/// For the execution plan to be valid, the following must hold:
/// - No Cycles
/// - The graph must be able to be topographically sorted such that all tasks that depend on a task
///     run before a task, and all tasks that finalize a task occur after said task
pub struct ExecutionGraph<E: ExecutableTask> {
    pub graph: DiGraph<E, TaskOrdering>,
    pub requested_tasks: Vec<TaskIdentifier>
}

struct TaskIdentifierGraph {
    graph: DiGraph<TaskIdentifier, TaskOrdering>,
    index_to_id: HashMap<TaskIdentifier, NodeIndex>,
}

impl TaskIdentifierGraph {
    fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            index_to_id: HashMap::new(),
        }
    }

    fn add_id(&mut self, id: TaskIdentifier) {
        let index = self.graph.add_node(id.clone());
        self.index_to_id.insert(id, index);
    }

    fn contains_id(&self, id: &TaskIdentifier) -> bool {
        self.index_to_id.contains_key(id)
    }

    fn add_task_ordering(
        &mut self,
        from_id: TaskIdentifier,
        to_id: TaskIdentifier,
        dependency_type: TaskOrdering,
    ) {
        let from = self.index_to_id[&from_id];
        let to = self.index_to_id[&to_id];
        self.graph.add_edge(from, to, dependency_type);
    }

    fn map_with<E: ExecutableTask>(
        self,
        container: &mut TaskContainer<E>,
        project: &Project<E>,
    ) -> Result<DiGraph<E, TaskOrdering>, ConstructionError> {
        let mut input = self.graph;

        let mut mapping = Vec::new();

        for node in input.node_indices() {
            let id = &input[node];

            let task = container.resolve_task(id.clone(), project)?;
            mapping.push((task, node));
        }

        let mut output: DiGraph<E, TaskOrdering> =
            DiGraph::with_capacity(input.node_count(), input.edge_count());
        let mut output_mapping = HashMap::new();
        for (exec, index) in mapping {
            let output_index = output.add_node(exec);
            output_mapping.insert(index, output_index);
        }

        for old_index in input.node_indices() {
            let new_index_from = output_mapping[&old_index];
            for outgoing in input.edges(old_index) {
                let weight = outgoing.weight().clone();
                let new_index_to = output_mapping[&outgoing.target()];
                output.add_edge(new_index_to, new_index_from, weight);
            }
        }
        Ok(output)
    }
}
