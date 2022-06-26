use crate::core::ConstructionError;
use assemble_core::identifier::TaskId;
use assemble_core::project::{Project, ProjectError};
use assemble_core::task::task_container::TaskContainer;
use assemble_core::task::TaskOrderingKind;
use assemble_core::{DefaultTask, Executable};
use petgraph::prelude::*;
use petgraph::visit::Visitable;
use std::collections::{HashMap, HashSet, VecDeque};



/// Resolves tasks
pub struct TaskResolver<'proj> {
    project: &'proj mut Project,
}

impl<'proj> TaskResolver<'proj> {
    /// Create a new instance of a task resolver for a project
    pub fn new(project: &'proj mut Project) -> Self {
        Self { project }
    }

    /// Try to find an identifier corresponding to the given id.
    ///
    /// Right now, only exact matches are allowed.
    pub fn try_find_identifier(&self, id: &str) -> Option<TaskId> {
        self.project
            .find_task_id(id)
            .ok()
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
    /// project.task::<Empty>("task2").unwrap().configure_with(|_, opts, _| {
    ///     opts.depend_on("task1");
    ///     Ok(())
    /// })
    /// ```
    pub fn to_execution_graph<I: IntoIterator<Item = &'proj TaskId>>(
        mut self,
        tasks: I,
    ) -> Result<ExecutionGraph<DefaultTask>, ConstructionError> {
        let mut task_id_graph = TaskIdentifierGraph::new();

        let mut task_container = self.project.task_container();

        let mut task_queue: VecDeque<TaskId> = VecDeque::new();
        let requested = tasks.into_iter().cloned().collect::<Vec<_>>();
        task_queue.extend(requested.clone());

        let mut visited = HashSet::new();

        while let Some(task_id) = task_queue.pop_front() {
            if visited.contains(&task_id) {
                continue;
            }

            if !task_id_graph.contains_id(&task_id) {
                task_id_graph.add_id(task_id.clone());
            }
            visited.insert(task_id.clone());

            let config_info = task_container.configure_task(task_id.clone(), self.project)?;
            println!("got configured info: {:#?}", config_info);
            for ordering in config_info.ordering {
                let next_id = &ordering.buildable;
                if !task_id_graph.contains_id(next_id) {
                    task_id_graph.add_id(next_id.clone());
                }
                task_queue.push_back(next_id.clone());
                task_id_graph.add_task_ordering(
                    task_id.clone(),
                    next_id.clone(),
                    ordering.ordering_type,
                );
            }
        }
        println!("Attempting to create execution graph.");
        let execution_graph = task_id_graph.map_with(&mut task_container, &self.project)?;
        Ok(ExecutionGraph {
            graph: execution_graph,
            requested_tasks: requested,
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
pub struct ExecutionGraph<E: Executable> {
    /// The task ordering graph
    pub graph: DiGraph<E, TaskOrderingKind>,
    /// Tasks requested
    pub requested_tasks: Vec<TaskId>,
}

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
        container: &mut TaskContainer<DefaultTask>,
        project: &Project,
    ) -> Result<DiGraph<DefaultTask, TaskOrderingKind>, ConstructionError> {
        let mut input = self.graph;

        let mut mapping = Vec::new();

        for node in input.node_indices() {
            let id = &input[node];

            let task = container.resolve_task(id.clone(), project)?;
            mapping.push((task, node));
        }

        let mut output: DiGraph<DefaultTask, TaskOrderingKind> =
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
