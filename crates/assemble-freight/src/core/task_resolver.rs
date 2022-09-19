use crate::core::ConstructionError;
use assemble_core::error::PayloadError;
use assemble_core::identifier::TaskId;
use assemble_core::project::buildable::Buildable;
use assemble_core::project::error::{ProjectError, ProjectResult};
use assemble_core::project::requests::TaskRequests;
use assemble_core::project::{Project, SharedProject};
use assemble_core::task::task_container::{FindTask, TaskContainer};
use assemble_core::task::{FullTask, TaskOrderingKind};
use colored::Colorize;
use petgraph::prelude::*;
use petgraph::visit::Visitable;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{Debug, Formatter};

/// Resolves tasks
pub struct TaskResolver {
    project: SharedProject,
}

impl TaskResolver {
    /// Create a new instance of a task resolver for a project
    pub fn new(project: &SharedProject) -> Self {
        Self {
            project: project.clone(),
        }
    }

    /// Try to find an identifier corresponding to the given id.
    ///
    /// Right now, only exact matches are allowed.
    pub fn try_find_identifier(&self, id: &str) -> ProjectResult<TaskId> {
        self.project.with(|p| p.find_task_id(id))
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
    /// use assemble_core::defaults::tasks::Empty;
    /// # let mut project = Project::temp(None);
    /// project.register_task::<Empty>("task1").unwrap();
    /// project.register_task::<Empty>("task2").unwrap().configure_with(|task, _| {
    ///     task.depends_on("task1");
    ///     Ok(())
    /// }).unwrap();
    /// ```
    pub fn to_execution_graph(
        self,
        tasks: TaskRequests,
    ) -> Result<ExecutionGraph, ConstructionError> {
        let mut task_id_graph = TaskIdentifierGraph::new();

        let mut task_queue: VecDeque<TaskId> = VecDeque::new();
        let requested = tasks.requested_tasks().iter().cloned().collect::<Vec<_>>();
        task_queue.extend(requested.clone());

        let mut visited = HashSet::new();

        while let Some(task_id) = task_queue.pop_front() {
            if visited.contains(&task_id) {
                trace!("task {task_id} already visited, skipping...");
                continue;
            }
            trace!("adding dependencies of {task_id} to task graph");

            if !task_id_graph.contains_id(&task_id) {
                trace!("adding {} to task graph", task_id);
                task_id_graph.add_id(task_id.clone());
            }
            visited.insert(task_id.clone());

            let config_info = self
                .project
                .task_container()
                .get_task(&task_id)?
                .resolve_shared(&self.project)?;

            trace!("got configured info: {:#?}", config_info);
            for ordering in config_info.ordering() {
                let buildable = ordering.buildable();
                let dependencies = self.project.with(|p| buildable.get_dependencies(p))?;

                debug!(
                    "[{:^20}] adding dependencies from {:?} -> {:#?}",
                    task_id.to_string().italic(),
                    buildable,
                    dependencies
                );

                for next_id in dependencies {
                    if !task_id_graph.contains_id(&next_id) {
                        trace!("adding {} to task graph", task_id);
                        task_id_graph.add_id(next_id.clone());
                    }

                    trace!(
                        "creating task dependency from {} to {} with kind {:?}",
                        task_id,
                        next_id,
                        ordering.ordering_kind()
                    );

                    trace!("adding {} to resolve queue", next_id);
                    task_queue.push_back(next_id.clone());
                    task_id_graph.add_task_ordering(
                        task_id.clone(),
                        next_id.clone(),
                        *ordering.ordering_kind(),
                    );
                    trace!("task_id_graph: {:#?}", task_id_graph);
                }
            }
        }
        debug!("Attempting to create execution graph.");
        let execution_graph = self
            .project
            .with(|project| task_id_graph.map_with(project.task_container(), project))?;
        Ok(ExecutionGraph {
            graph: execution_graph,
            requested_tasks: tasks,
        })
    }
}
/// The Execution Plan provides a plan of executable tasks that
/// the task executor can execute.
///
/// For the execution plan to be valid, the following must hold:
/// - No Cycles
/// - The graph must be able to be topographically sorted such that all tasks that depend on a task
///     run before a task, and all tasks that finalize a task occur after said task
#[derive(Debug)]
pub struct ExecutionGraph {
    /// The task ordering graph
    pub graph: DiGraph<Box<dyn FullTask>, TaskOrderingKind>,
    /// Tasks requested
    pub requested_tasks: TaskRequests,
}

// impl Debug for ExecutionGraph {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("ExecutionGraph")
//             .field("requested_tasks", &self.requested_tasks)
//             .finish_non_exhaustive()
//     }
// }
#[derive(Debug)]
struct TaskIdentifierGraph {
    graph: DiGraph<TaskId, TaskOrderingKind>,
    index_to_id: HashMap<TaskId, NodeIndex>,
}

impl TaskIdentifierGraph {
    fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            index_to_id: HashMap::new(),
        }
    }

    fn add_id(&mut self, id: TaskId) {
        let index = self.graph.add_node(id.clone());
        self.index_to_id.insert(id, index);
    }

    fn contains_id(&self, id: &TaskId) -> bool {
        self.index_to_id.contains_key(id)
    }

    fn add_task_ordering(
        &mut self,
        from_id: TaskId,
        to_id: TaskId,
        dependency_type: TaskOrderingKind,
    ) {
        let from = self.index_to_id[&from_id];
        let to = self.index_to_id[&to_id];
        self.graph.add_edge(from, to, dependency_type);
    }

    fn map_with(
        self,
        container: &TaskContainer,
        project: &Project,
    ) -> Result<DiGraph<Box<dyn FullTask>, TaskOrderingKind>, ConstructionError> {
        trace!("creating digraph from TaskIdentifierGraph");
        let input = self.graph;

        let mut mapping = Vec::new();

        for node in input.node_indices() {
            let id = &input[node];

            let task = container.get_task(id)?;
            mapping.push((task, node));
        }

        let mut output: DiGraph<Box<dyn FullTask>, TaskOrderingKind> =
            DiGraph::with_capacity(input.node_count(), input.edge_count());
        let mut output_mapping = HashMap::new();

        for (mut exec, index) in mapping {
            let output_index = output.add_node(exec.resolve(project)?);
            output_mapping.insert(index, output_index);
        }

        for old_index in input.node_indices() {
            let new_index_from = output_mapping[&old_index];
            for outgoing in input.edges(old_index) {
                let weight = outgoing.weight().clone();
                let new_index_to = output_mapping[&outgoing.target()];
                output.add_edge(new_index_from, new_index_to, weight);
            }
        }
        Ok(output)
    }
}
