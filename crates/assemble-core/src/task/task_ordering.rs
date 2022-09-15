use crate::__export::TaskId;
use crate::project::buildable::{Buildable, IntoBuildable};
use crate::project::error::ProjectError;
use crate::Project;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use crate::prelude::ProjectResult;

/// Represents some task ordering.
#[derive(Clone)]
pub struct TaskOrdering {
    buildable: Arc<dyn Buildable>,
    ordering_kind: TaskOrderingKind,
}

impl Debug for TaskOrdering {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}({:?})", self.ordering_kind, self.buildable)
    }
}

impl Buildable for TaskOrdering {
    fn get_dependencies(&self, project: &Project) -> ProjectResult<HashSet<TaskId>> {
        self.buildable.get_dependencies(project)
    }
}

impl TaskOrdering {
    /// Create a depends_on ordering
    pub fn depends_on<B: IntoBuildable>(buildable: B) -> Self
    where
        B::Buildable: 'static,
    {
        Self {
            buildable: Arc::new(buildable.into_buildable()),
            ordering_kind: TaskOrderingKind::DependsOn,
        }
    }

    pub fn buildable(&self) -> &Arc<dyn Buildable> {
        &self.buildable
    }
    pub fn ordering_kind(&self) -> &TaskOrderingKind {
        &self.ordering_kind
    }
}
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum TaskOrderingKind {
    DependsOn,
    FinalizedBy,
    RunsBefore,
    RunsAfter,
}
