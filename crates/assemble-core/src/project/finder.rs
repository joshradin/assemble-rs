//! Helpers for subproject handling

use crate::identifier::ID_SEPARATOR;
use crate::prelude::ProjectId;
use crate::project::{GetProjectId, SharedProject};
use itertools::Itertools;
use std::collections::VecDeque;

/// Finds a sub project.
///
/// # Project Resolution
/// From a given *base* project, a path is resolved as follows:
/// - an empty path (`""`) refers to the base project.
/// - a path starting with a `":"` searches starting from the `root` project.
/// - A path starting with `:<root project name>` is equivalent to just `":"`.
/// - each component after that, which are seperated by a `":"`, is a child of the current search
///
/// For example, in this given structure with a search starting pointed marked with `<--`,
/// ```text
/// root:
///   - child1: <--
///     - child2:
///   - child3:
/// ```
///
/// To access `child2`, you can either use `child2`, `:child1:child2`, or `:root:child1:child2`.
/// Meanwhile, `child3` can only be accessed via `:child3`, or `:root:child3`.
#[derive(Debug)]
pub struct ProjectFinder {
    project: SharedProject,
}

impl ProjectFinder {
    /// Creates a new project finder
    pub fn new(project: &SharedProject) -> Self {
        Self {
            project: project.clone(),
        }
    }
    /// Tries to find a project relative to this one from a project id.
    ///
    /// For more info on how finding works, check out the definition of the [`ProjectFinder`](ProjectFinder)
    pub fn find<'a, S: Into<ProjectPath<'a>>>(&self, id: S) -> Option<SharedProject> {
        let path = id.into();

        let mut project_ptr = Some(self.project.clone());
        let mut at_root = |ptr: &Option<SharedProject>| -> bool {
            ptr.as_ref().map(|p| p.is_root()).unwrap_or(false)
        };

        for component in path.components() {
            match component {
                PathComponent::Root => {
                    project_ptr = Some(self.project.with(|p| p.root_project()));
                }
                PathComponent::Normal(normal) => {
                    if at_root(&project_ptr) && project_ptr.as_ref().unwrap().project_id() == normal
                    {
                        continue;
                    }
                    project_ptr = project_ptr.and_then(|s| s.get_subproject(normal).ok());
                }
            }
        }

        project_ptr
    }
}

/// Represents a path to a project
#[derive(Debug)]
pub struct ProjectPath<'a> {
    components: Vec<PathComponent<'a>>,
}

impl<'a> ProjectPath<'a> {
    /// Create a new path from a string
    pub fn new(mut path: &'a str) -> Self {
        let mut comp = vec![];
        if path.starts_with(ID_SEPARATOR) {
            comp.push(PathComponent::Root);
            path = &path[1..];
        }

        comp.extend(
            path.split_terminator(ID_SEPARATOR)
                .map(|s| PathComponent::Normal(s)),
        );

        Self { components: comp }
    }

    /// Gets the components of the path
    pub fn components(&self) -> PathComponents<'_> {
        PathComponents {
            comps: &self.components,
            index: 0,
        }
    }
}

impl<'a> IntoIterator for &'a ProjectPath<'a> {
    type Item = PathComponent<'a>;
    type IntoIter = PathComponents<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.components()
    }
}

/// A component of a project path
#[derive(Debug, Clone)]
pub enum PathComponent<'a> {
    Root,
    Normal(&'a str),
}

impl<'a> Into<ProjectPath<'a>> for &'a str {
    fn into(self) -> ProjectPath<'a> {
        ProjectPath::new(self)
    }
}

impl<'a> Into<ProjectPath<'a>> for &'a ProjectId {
    fn into(self) -> ProjectPath<'a> {
        let mut components = vec![];
        for part in self.iter() {
            components.push(PathComponent::Normal(part));
        }
        ProjectPath { components }
    }
}

/// An iterator over the components of the project path
#[derive(Debug)]
pub struct PathComponents<'a> {
    comps: &'a Vec<PathComponent<'a>>,
    index: usize,
}

impl<'a> Iterator for PathComponents<'a> {
    type Item = PathComponent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let out = self.comps.get(self.index).cloned();
        if out.is_some() {
            self.index += 1;
        }
        out
    }
}

/// Represents a path to a task
#[derive(Debug)]
pub struct TaskPath<'a> {
    project: ProjectPath<'a>,
    task: &'a str,
}

impl<'a> TaskPath<'a> {
    /// Create a new task path from a string
    pub fn new(path: &'a str) -> Self {
        let mut sep: VecDeque<&'a str> = path.rsplitn(1, ID_SEPARATOR).collect();
        let task = sep.pop_front().expect("one always expected");
        let rest = sep.pop_front().unwrap();
        let project_path = ProjectPath::new(rest);
        Self {
            project: project_path,
            task,
        }
    }

    /// Gets the project part of the task path, if it exists.
    pub fn project(&self) -> &ProjectPath<'a> {
        &self.project
    }

    /// Gets the task component itself
    pub fn task(&self) -> &'a str {
        self.task
    }
}

/// Similar to the project finder, but for tasks.
#[derive(Debug)]
pub struct TaskFinder {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::dev::quick_create;
    use crate::Project;
    use toml::toml;

    fn init() -> SharedProject {
        quick_create(
            r"
        root:
          - sub1:
               - sub2:
        ",
        )
        .expect("couldn't create a project")
    }

    #[test]
    fn project_relative_search() {
        let project = init();
        let sub1 = project.get_subproject("sub1").unwrap();
        let finder = ProjectFinder::new(&sub1);

        assert_eq!(finder.find("").unwrap().project_id(), ":root:sub1");
        assert_eq!(finder.find("sub2").unwrap().project_id(), ":root:sub1:sub2");
    }

    #[test]
    fn project_absolute_search() {
        let project = init();
        let sub1 = project.get_subproject("sub1").unwrap();
        let finder = ProjectFinder::new(&sub1);
        // start from sub 1 to make it more "confirmed"

        assert_eq!(finder.find(":").unwrap().project_id(), ":root");
        assert_eq!(finder.find(":root").unwrap().project_id(), ":root");
        assert_eq!(finder.find(":sub1").unwrap().project_id(), ":root:sub1");
        assert_eq!(
            finder.find(":sub1:sub2").unwrap().project_id(),
            ":root:sub1:sub2"
        );
        assert!(
            finder.find(":sub2").is_none(),
            "sub2 is not a child of the root"
        );
    }
}
