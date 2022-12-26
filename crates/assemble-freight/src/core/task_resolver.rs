use crate::core::ConstructionError;

use crate::consts::EXEC_GRAPH_LOG_LEVEL;
use assemble_core::identifier::{ProjectId, TaskId};
use assemble_core::project::buildable::Buildable;
use assemble_core::project::error::ProjectResult;
use assemble_core::project::requests::TaskRequests;
use assemble_core::project::{GetProjectId, Project};
use assemble_core::task::task_container::TaskContainer;
use assemble_core::task::{FullTask, TaskOrderingKind};
use colored::Colorize;
use petgraph::prelude::*;

use assemble_core::dependencies::project_dependency::ProjectDependencyPlugin;
use assemble_core::error::PayloadError;
use assemble_core::prelude::ProjectError;
use assemble_core::project::finder::{
    ProjectFinder, ProjectPathBuf, TaskFinder, TaskPath, TaskPathBuf,
};
use assemble_core::project::shared::SharedProject;
use assemble_core::startup::execution_graph::{ExecutionGraph, SharedAnyTask};
use itertools::Itertools;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Debug;
use std::sync::Arc;

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

    pub fn find_task(
        &self,
        task_id: &TaskId,
    ) -> Result<Box<dyn FullTask>, PayloadError<ConstructionError>> {
        let project_id = task_id.project_id();
        match project_id {
            None => {
                panic!("task {} has no parent", task_id);
            }
            Some(project) => {
                let mut ptr = self.project.clone();
                let mut iter = project.iter();
                let first = iter.next().unwrap();
                if ptr.project_id() != first {
                    return Err(
                        ConstructionError::ProjectError(ProjectError::NoSharedProjectSet).into(),
                    );
                }
                for id in iter {
                    ptr = ptr.get_subproject(id).map_err(PayloadError::into)?;
                }

                let config_info = ptr
                    .get_task(task_id)
                    .map_err(PayloadError::into)?
                    .resolve_shared(&self.project)
                    .map_err(PayloadError::into)?;

                Ok(config_info)
            }
        }
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
    ) -> Result<ExecutionGraph, PayloadError<ConstructionError>> {
        let mut task_id_graph = TaskIdentifierGraph::new();

        let mut task_queue: VecDeque<TaskId> = VecDeque::new();
        let requested = tasks.requested_tasks().to_vec();
        task_queue.extend(requested);
        log!(
            EXEC_GRAPH_LOG_LEVEL,
            "task queue at start: {:?}",
            task_queue
        );

        let mut visited = HashSet::new();

        while let Some(task_id) = task_queue.pop_front() {
            if visited.contains(&task_id) {
                log!(
                    EXEC_GRAPH_LOG_LEVEL,
                    "task {task_id} already visited, skipping..."
                );
                continue;
            }

            if !task_id_graph.contains_id(&task_id) {
                log!(EXEC_GRAPH_LOG_LEVEL, "adding {} to task graph", task_id);
                task_id_graph.add_id(task_id.clone());
            }
            visited.insert(task_id.clone());

            let config_info = self.find_task(&task_id)?;

            log!(
                EXEC_GRAPH_LOG_LEVEL,
                "got configured info: {:#?}",
                config_info
            );
            for ordering in config_info.ordering() {
                let buildable = ordering.buildable();
                let dependencies = self
                    .project
                    .with(|p| buildable.get_dependencies(p))
                    .map_err(PayloadError::into)?;

                log!(
                    EXEC_GRAPH_LOG_LEVEL,
                    "[{:^20}] adding dependencies from {:?} -> {:#?}",
                    task_id.to_string().italic(),
                    buildable,
                    dependencies
                );

                for next_id in dependencies {
                    if !task_id_graph.contains_id(&next_id) {
                        log!(EXEC_GRAPH_LOG_LEVEL, "adding {} to task graph", task_id);
                        task_id_graph.add_id(next_id.clone());
                    }

                    log!(
                        EXEC_GRAPH_LOG_LEVEL,
                        "creating task dependency from {} to {} with kind {:?}",
                        task_id,
                        next_id,
                        ordering.ordering_kind()
                    );

                    log!(EXEC_GRAPH_LOG_LEVEL, "adding {} to resolve queue", next_id);
                    task_queue.push_back(next_id.clone());
                    task_id_graph.add_task_ordering(
                        task_id.clone(),
                        next_id.clone(),
                        *ordering.ordering_kind(),
                    );
                    log!(EXEC_GRAPH_LOG_LEVEL, "task_id_graph: {:#?}", task_id_graph);
                }
            }
        }
        debug!("Attempting to create execution graph.");
        let execution_graph = task_id_graph.map_with(self.project.clone())?;
        Ok(ExecutionGraph::new(execution_graph, tasks))
    }
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
        project: SharedProject,
    ) -> Result<DiGraph<SharedAnyTask, TaskOrderingKind>, PayloadError<ConstructionError>> {
        trace!("creating digraph from TaskIdentifierGraph");
        let input = self.graph;

        let mut mapping = Vec::new();

        let finder = ProjectFinder::new(&project);

        for node in input.node_indices() {
            let id = &input[node];
            let project: ProjectPathBuf = id.project_id().unwrap().into();

            let project = finder
                .find(&project)
                .unwrap_or_else(|| panic!("no project found for name {:?}", project));

            let mut task = project.get_task(id).map_err(PayloadError::into)?;
            let task = task.resolve_shared(&project).map_err(PayloadError::into)?;
            mapping.push((task, node));
        }

        let mut output: DiGraph<SharedAnyTask, TaskOrderingKind> =
            DiGraph::with_capacity(input.node_count(), input.edge_count());
        let mut output_mapping = HashMap::new();

        for (exec, index) in mapping {
            let output_index = output.add_node(Arc::new(RwLock::new(exec)));
            output_mapping.insert(index, output_index);
        }

        for old_index in input.node_indices() {
            let new_index_from = output_mapping[&old_index];
            for outgoing in input.edges(old_index) {
                let weight = *outgoing.weight();
                let new_index_to = output_mapping[&outgoing.target()];
                output.add_edge(new_index_from, new_index_to, weight);
            }
        }
        Ok(output)
    }
}
