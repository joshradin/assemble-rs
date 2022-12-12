use petgraph::prelude::*;

use crate::unstable::text_factory::graph::PrettyGraph;
use ptree::{IndentChars, PrintConfig};
use std::fmt;
use std::fmt::Write as _;
use std::fmt::{Debug, Display, Formatter};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// A project descriptor is used to define projects.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProjectDescriptor {
    build_file: ProjectDescriptorLocation,
    name: String,
}

impl Display for ProjectDescriptor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({:?})", self.name, self.build_file)
    }
}

impl ProjectDescriptor {
    /// creates a new project descriptor
    fn new<S: AsRef<str>>(name: S, build_file: ProjectDescriptorLocation) -> Self {
        let name = name.as_ref().to_string();
        Self { build_file, name }
    }

    /// if this only has the directory known, this sets the file name
    fn set_file_name(&mut self, file_name: &str) {
        let file_path = if let ProjectDescriptorLocation::KnownDirectory(dir) = &self.build_file {
            dir.join(file_name)
        } else {
            return;
        };

        self.build_file = ProjectDescriptorLocation::KnownFile(file_path);
    }

    /// Gets the name of the project
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Sets the name of the project
    pub fn set_name(&mut self, name: impl AsRef<str>) {
        self.name = name.as_ref().to_string();
    }

    /// Gets the build file associated with this project, if known
    pub fn build_file(&self) -> Option<&Path> {
        match &self.build_file {
            ProjectDescriptorLocation::KnownFile(f) => Some(f),
            ProjectDescriptorLocation::KnownDirectory(_) => None,
        }
    }

    /// Gets the directory this project is contained in
    pub fn directory(&self) -> &Path {
        match &self.build_file {
            ProjectDescriptorLocation::KnownFile(file) => file.parent().unwrap(),
            ProjectDescriptorLocation::KnownDirectory(dir) => dir.as_ref(),
        }
    }

    /// Checks if this project descriptor is contained in this directory
    pub fn matches_dir(&self, path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        match &self.build_file {
            ProjectDescriptorLocation::KnownFile(f) => match f.parent() {
                Some(parent) => parent.ends_with(path),
                None => path == Path::new(""),
            },
            ProjectDescriptorLocation::KnownDirectory(d) => d.ends_with(path),
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
enum ProjectDescriptorLocation {
    KnownFile(PathBuf),
    KnownDirectory(PathBuf),
}

impl Debug for ProjectDescriptorLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ProjectDescriptorLocation::KnownFile(file) => {
                write!(f, "{:?}", file)
            }
            ProjectDescriptorLocation::KnownDirectory(d) => {
                write!(f, "{:?}", d.join("?"))
            }
        }
    }
}

/// Describes the project graph
#[derive(Debug)]
pub struct ProjectGraph {
    graph: DiGraph<ProjectDescriptor, ()>,
    project_dir: PathBuf,
    root_project: NodeIndex,
    default_build_script_file: Option<String>,
}

impl ProjectGraph {
    /// Creates a new project graph with a pre-initialized root project
    pub(crate) fn new<P: AsRef<Path>>(project_dir: P) -> Self {
        let project_name = project_dir
            .as_ref()
            .file_name()
            .unwrap_or_else(|| panic!("{:?} has no file name", project_dir.as_ref()))
            .to_str()
            .unwrap_or_else(|| {
                panic!(
                    "{:?} file name can not be represented as a utf-8 string",
                    project_dir.as_ref()
                )
            });

        let root_project = ProjectDescriptor::new(
            project_name,
            ProjectDescriptorLocation::KnownDirectory(project_dir.as_ref().to_path_buf()),
        );
        let mut graph = DiGraph::new();
        let idx = graph.add_node(root_project);
        Self {
            graph,
            project_dir: project_dir.as_ref().to_path_buf(),
            root_project: idx,
            default_build_script_file: None,
        }
    }

    /// Gets the root project descriptor
    pub fn root_project(&self) -> &ProjectDescriptor {
        &self.graph[self.root_project]
    }

    /// Gets a mutable reference to the root project descriptor
    pub fn root_project_mut(&mut self) -> &mut ProjectDescriptor {
        &mut self.graph[self.root_project]
    }

    pub fn set_default_build_file_name(&mut self, name: &str) {
        if self.default_build_script_file.is_some() {
            panic!(
                "default build script file name already set to {:?}",
                self.default_build_script_file.as_ref().unwrap()
            );
        }

        self.default_build_script_file = Some(name.to_string());
        for node in self.graph.node_indices() {
            self.graph[node].set_file_name(name);
        }
    }

    /// Adds a child project to the root project
    pub fn project<S: AsRef<str>, F: FnOnce(&mut ProjectBuilder)>(
        &mut self,
        path: S,
        configure: F,
    ) {
        let path = path.as_ref();
        debug!("adding project with path {:?}", path);
        let mut builder = ProjectBuilder::new(&self.project_dir, path.to_string());
        (configure)(&mut builder);
        self.add_project_from_builder(self.root_project, builder);
    }

    /// Adds a child project to some other project
    fn add_project_from_builder(&mut self, parent: NodeIndex, builder: ProjectBuilder) {
        let ProjectBuilder {
            name,
            dir,
            children,
        } = builder;

        let location = match &self.default_build_script_file {
            None => ProjectDescriptorLocation::KnownDirectory(dir),
            Some(s) => ProjectDescriptorLocation::KnownFile(dir.join(s)),
        };
        let pd = ProjectDescriptor::new(name, location);

        let node = self.graph.add_node(pd);
        self.graph.add_edge(parent, node, ());

        for child_builder in children {
            self.add_project_from_builder(node, child_builder);
        }
    }

    /// Find a project by path
    pub fn find_project<P: AsRef<Path>>(&self, path: P) -> Option<&ProjectDescriptor> {
        self.graph
            .node_indices()
            .find(|&idx| self.graph[idx].matches_dir(&path))
            .map(|idx| &self.graph[idx])
    }

    /// Find a project by path
    pub fn find_project_mut<P: AsRef<Path>>(&mut self, path: P) -> Option<&mut ProjectDescriptor> {
        self.graph
            .node_indices()
            .find(|&idx| self.graph[idx].matches_dir(&path))
            .map(|idx| &mut self.graph[idx])
    }

    /// Gets the child project of a given project
    pub fn children_projects(
        &self,
        proj: &ProjectDescriptor,
    ) -> impl IntoIterator<Item = &ProjectDescriptor> {
        self.graph
            .node_indices()
            .find(|&idx| &self.graph[idx] == proj)
            .into_iter()
            .map(|index| {
                self.graph
                    .neighbors(index)
                    .into_iter()
                    .map(|neighbor| &self.graph[neighbor])
            })
            .flatten()
    }
}

impl Display for ProjectGraph {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let pretty = PrettyGraph::new(&self.graph, self.root_project);
        write!(f, "{}", pretty)
    }
}

/// Helps to build a project
pub struct ProjectBuilder {
    name: String,
    dir: PathBuf,
    children: Vec<ProjectBuilder>,
}

impl ProjectBuilder {
    fn new(parent_dir: &Path, name: String) -> Self {
        let mut dir = parent_dir.join(&name);
        trace!("parent dir: {parent_dir:?}");
        trace!("project name: {name:?}");
        trace!("using dir: {dir:?}");
        Self {
            name,
            dir,
            children: vec![],
        }
    }

    /// Sets the name of this project
    pub fn set_name(&mut self, name: impl AsRef<str>) {
        self.name = name.as_ref().to_string();
    }

    /// Sets the directory of this project. by default, the directory is the parent projects
    /// directory + name
    pub fn set_dir(&mut self, path: impl AsRef<Path>) {
        self.dir = path.as_ref().to_path_buf();
    }

    /// Adds a child project to this project. `path` is relative to this project, and should
    /// be written as a simple identifier
    pub fn project<S: AsRef<str>, F: FnOnce(&mut ProjectBuilder)>(
        &mut self,
        path: S,
        configure: F,
    ) {
        let path = path.as_ref();
        let mut builder = ProjectBuilder::new(&self.dir, path.to_string());
        (configure)(&mut builder);
        self.children.push(builder);
    }
}

#[cfg(test)]
mod tests {
    use crate::startup::initialization::ProjectGraph;

    use std::path::PathBuf;

    #[test]
    fn print_graph() {
        let path = PathBuf::from("assemble");
        let mut graph = ProjectGraph::new(path);

        graph.project("list", |builder| {
            builder.project("linked", |_| {});
            builder.project("array", |_| {});
        });
        graph.project("map", |_| {});

        println!("{}", graph);
    }

    #[test]
    fn can_set_default_build_name() {
        let path = PathBuf::from("assemble");
        let mut graph = ProjectGraph::new(path);
        graph.set_default_build_file_name("build.assemble");

        println!("{}", graph);
        assert_eq!(
            graph.root_project().build_file(),
            Some(&*PathBuf::from_iter(["assemble", "build.assemble"]))
        )
    }

    #[test]
    fn can_find_project() {
        let path = PathBuf::from("assemble");
        let mut graph = ProjectGraph::new(path);

        graph.project("list", |builder| {
            builder.project("linked", |_| {});
            builder.project("array", |_| {});
        });
        graph.project("map", |b| {
            b.project("set", |_| {});
            b.project("ordered", |_| {});
            b.project("hashed", |_| {});
        });

        println!("graph: {:#}", graph);

        assert!(graph.find_project("assemble/map/hashed").is_some());
        assert!(graph.find_project("assemble/list/array").is_some());
        assert!(graph.find_project("assemble/list/garfunkle").is_none());
    }
}
