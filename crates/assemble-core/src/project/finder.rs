//! Helpers for subproject handling

use crate::identifier::{TaskId, ID_SEPARATOR};
use crate::prelude::ProjectId;
use crate::project;
use crate::project::shared::SharedProject;
use crate::project::{GetProjectId, ProjectError, ProjectResult};
use crate::task::HasTaskId;
use itertools::Itertools;
use std::borrow::Borrow;
use std::collections::VecDeque;
use std::fmt::{Display, Formatter};
use std::iter::FusedIterator;
use std::mem::transmute;
use std::ops::Deref;

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
    pub fn find<S: AsRef<ProjectPath>>(&self, id: S) -> Option<SharedProject> {
        let path = id.as_ref();

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
#[derive(Debug, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct ProjectPath {
    path: str,
}

impl ProjectPath {
    /// Create a new path from a string
    pub fn new(path: &str) -> &Self {
        unsafe { transmute(path) }
    }

    /// Checks whether this an empty path
    pub fn is_empty(&self) -> bool {
        self.components().count() == 0
    }

    /// Gets the components of the path
    pub fn components(&self) -> PathComponents<'_> {
        let mut comp = vec![];
        let mut path = &self.path;
        if path.starts_with(ID_SEPARATOR) {
            comp.push(PathComponent::Root);
            path = &path[1..];
        }

        comp.extend(
            path.split_terminator(ID_SEPARATOR)
                .map(|s| PathComponent::Normal(s)),
        );

        PathComponents {
            comps: comp,
            index: 0,
        }
    }
}

impl AsRef<ProjectPath> for ProjectPath {
    fn as_ref(&self) -> &ProjectPath {
        self
    }
}

impl Borrow<ProjectPath> for ProjectPathBuf {
    fn borrow(&self) -> &ProjectPath {
        self.as_ref()
    }
}

impl ToOwned for ProjectPath {
    type Owned = ProjectPathBuf;

    fn to_owned(&self) -> Self::Owned {
        ProjectPathBuf::new(self.path.to_string())
    }
}

impl<'a> IntoIterator for &'a ProjectPath {
    type Item = PathComponent<'a>;
    type IntoIter = PathComponents<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.components()
    }
}

/// A component of a project path
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PathComponent<'a> {
    Root,
    Normal(&'a str),
}

/// An owned version of the project path
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ProjectPathBuf {
    string: String,
}

impl ProjectPathBuf {
    /// creates a new project path
    pub fn new(path: String) -> ProjectPathBuf {
        ProjectPathBuf { string: path }
    }
}

impl From<&ProjectPath> for ProjectPathBuf {
    fn from(value: &ProjectPath) -> Self {
        ProjectPathBuf::new(value.path.to_string())
    }
}

impl AsRef<ProjectPath> for ProjectPathBuf {
    fn as_ref(&self) -> &ProjectPath {
        ProjectPath::new(&self.string)
    }
}

impl AsRef<ProjectPath> for &str {
    fn as_ref(&self) -> &ProjectPath {
        ProjectPath::new(*self)
    }
}

impl AsRef<ProjectPath> for String {
    fn as_ref(&self) -> &ProjectPath {
        ProjectPath::new(&self)
    }
}

impl<S: AsRef<str>> From<S> for ProjectPathBuf {
    fn from(value: S) -> Self {
        Self::new(value.as_ref().to_string())
    }
}

impl From<ProjectId> for ProjectPathBuf {
    fn from(value: ProjectId) -> Self {
        ProjectPathBuf::new(value.to_string())
    }
}

impl Deref for ProjectPathBuf {
    type Target = ProjectPath;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

/// An iterator over the components of the project path
#[derive(Debug)]
pub struct PathComponents<'a> {
    comps: Vec<PathComponent<'a>>,
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

impl FusedIterator for PathComponents<'_> {}

/// Represents a path to a task
#[derive(Debug, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct TaskPath {
    path: str,
}

impl TaskPath {
    /// Create a new task path from a string
    pub fn new(path: &str) -> &Self {
        unsafe { transmute(path) }
    }

    fn split(&self) -> (&ProjectPath, &str) {
        let mut sep: VecDeque<&str> = self.path.rsplitn(2, ID_SEPARATOR).collect();
        let task = sep.pop_front().expect("one always expected");
        if let Some(rest) = sep.pop_front() {
            let project_path = if rest.is_empty() {
                ProjectPath::new(":")
            } else {
                ProjectPath::new(rest)
            };
            (project_path, task)
        } else {
            (ProjectPath::new(""), task)
        }
    }

    /// Gets the project part of the task path, if it exists.
    pub fn project(&self) -> &ProjectPath {
        self.split().0
    }

    /// Gets the task component itself
    pub fn task(&self) -> &str {
        self.split().1
    }
}

impl ToOwned for TaskPath {
    type Owned = TaskPathBuf;

    fn to_owned(&self) -> Self::Owned {
        TaskPathBuf::from(self)
    }
}

impl Borrow<TaskPath> for TaskPathBuf {
    fn borrow(&self) -> &TaskPath {
        self.as_ref()
    }
}

impl AsRef<TaskPath> for TaskPath {
    fn as_ref(&self) -> &TaskPath {
        self
    }
}
impl AsRef<str> for TaskPath {
    fn as_ref(&self) -> &str {
        &self.path
    }
}

impl Display for TaskPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.path)
    }
}

/// An owned version of a [`TaskPath`](TaskPath).
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TaskPathBuf {
    src: String,
}

impl TaskPathBuf {
    /// Create a new task path buf from a string
    pub fn new(src: String) -> Self {
        Self { src }
    }
}

impl AsRef<str> for TaskPathBuf {
    fn as_ref(&self) -> &str {
        &self.src
    }
}

impl AsRef<TaskPath> for TaskPathBuf {
    fn as_ref(&self) -> &TaskPath {
        TaskPath::new(&self.src)
    }
}

impl AsRef<TaskPath> for &str {
    fn as_ref(&self) -> &TaskPath {
        TaskPath::new(*self)
    }
}

impl AsRef<TaskPath> for String {
    fn as_ref(&self) -> &TaskPath {
        TaskPath::new(&self)
    }
}

impl From<&TaskPath> for TaskPathBuf {
    fn from(value: &TaskPath) -> Self {
        TaskPathBuf::new(value.path.to_string())
    }
}

impl From<String> for TaskPathBuf {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for TaskPathBuf {
    fn from(value: &str) -> Self {
        Self::new(value.to_string())
    }
}

impl From<TaskId> for TaskPathBuf {
    fn from(value: TaskId) -> Self {
        Self::from(value.to_string())
    }
}

impl Deref for TaskPathBuf {
    type Target = TaskPath;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

/// Similar to the project finder, but for tasks.
#[derive(Debug)]
pub struct TaskFinder {
    project: SharedProject,
}

impl TaskFinder {
    /// Creates a new task finder from a given base project
    pub fn new(project: &SharedProject) -> Self {
        Self {
            project: project.clone(),
        }
    }

    /// Tries to find all tasks
    pub fn find<T: AsRef<TaskPath>>(&self, task_path: T) -> ProjectResult<Option<Vec<TaskId>>> {
        let (project, task) = task_path.as_ref().split();
        trace!(
            "searching for ({:?}, {:?}) from {}",
            project,
            task,
            self.project
        );
        let proj_finder = ProjectFinder::new(&self.project);
        let proj = proj_finder
            .find(project)
            .ok_or(ProjectError::ProjectNotFound(project.to_owned()))?;
        trace!("found proj: {}", proj);

        let mut output = vec![];

        if let Ok(task_id) = proj.task_id_factory().create(task) {
            trace!("checking if {} exists", task_id);
            if let Ok(task) = proj.get_task(&task_id) {
                if project.is_empty() && !task.only_current() {
                    output.push(task.task_id());
                } else {
                    trace!("exiting immediately with {}", task.task_id());
                    return Ok(Some(vec![task.task_id()]));
                }
            }
        }

        for registered_task in proj.task_container().get_tasks() {
            trace!("registered task: {}", registered_task);
            // todo:
        }

        if project.is_empty() {
            self.project.with(|p| {
                for subproject in p.subprojects() {
                    let finder = TaskFinder::new(subproject);
                    if let Ok(Some(tasks)) = finder.find(task) {
                        output.extend(tasks);
                    }
                }
            });
        }

        if output.is_empty() {
            Ok(None)
        } else {
            Ok(Some(output))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::defaults::tasks::Empty;
    use crate::project::dev::quick_create;
    use crate::Project;
    use std::cell::{Cell, Ref, RefCell};
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

    #[test]
    fn collect_relative_tasks() -> ProjectResult {
        let project = quick_create(
            r"
        parent:
            - mid1:
                - child1
                - child2
            - mid2:
                - child4
    ",
        )?;

        let mut count = RefCell::new(0_usize);
        project.allprojects_mut(|project| {
            project
                .task_container_mut()
                .register_task::<Empty>("taskName")
                .expect("couldnt register task");
            *count.borrow_mut() += 1;
        });

        let finder = TaskFinder::new(&project);
        let found = finder.find("taskName").unwrap().unwrap_or_default();
        println!("found: {:#?}", found);

        assert_eq!(
            found.len(),
            *count.borrow(),
            "all registered tasks of taskName should be found"
        );

        Ok(())
    }

    #[test]
    fn abs_works() -> ProjectResult {
        let project = quick_create(
            r"
        parent:
            - mid1:
                - child1
                - child2
    ",
        )?;

        let mut count = RefCell::new(0_usize);
        project.allprojects_mut(|project| {
            project
                .task_container_mut()
                .register_task::<Empty>("taskName")
                .expect("couldnt register task");
            *count.borrow_mut() += 1;
        });

        let finder = TaskFinder::new(&project.get_subproject(":mid1")?);
        let found = finder.find(":taskName").unwrap().unwrap_or_default();
        println!("found: {:#?}", found);

        assert_eq!(found.len(), 1, "only one returned");
        assert_eq!(&found[0], &TaskId::new(":parent:taskName").unwrap());

        Ok(())
    }
}
