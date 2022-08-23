//! The settings configuration

use std::fmt::Formatter;
use std::path::{Path, PathBuf};

use crate::build_logic::plugin::script::languages::YamlLang;
use crate::build_logic::plugin::script::{BuildScript, ScriptingLang};
use serde::de::{Error, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use assemble_core::prelude::ProjectId;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    name: String,
    /// Project definitions
    #[serde(default)]
    projects: Vec<ProjectDefinition>,
}

impl Settings {

    /// The name of the root project
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets the build script files associated with this settings. The first path is the root project
    pub fn projects(&self, root_dir: &Path) -> Vec<DefinedProject> {

        let mut output: Vec<_> = self.projects
                      .iter()
                      .flat_map(|s| s.script_files(root_dir, self.name.to_string()))
                      .collect();
        if let Some(script) = YamlLang.find_build_script(root_dir) {
            output.insert(0, DefinedProject::new(self.name.clone(), script, None));
        }
        output
    }
}

/// A located project represents a project with a name and a path
#[derive(Debug)]
pub struct DefinedProject {
    name: String,
    path: PathBuf,
    parent: Option<String>
}

impl DefinedProject {
    fn new(name: String, path: PathBuf, parent: Option<String>) -> Self {
        Self { name, path, parent }
    }


    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn parent(&self) -> Option<&str> {
        self.parent.as_deref()
    }

}


#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ProjectDefinition {
    Simple(String),
    Adv {
        name: String,
        path: Option<PathBuf>,
        projects: Option<Vec<ProjectDefinition>>,
    },
}

impl ProjectDefinition {
    /// Gets the build script files associated with this settings
    fn script_files(&self, root_dir: &Path, parent: impl Into<Option<String>>) -> Vec<DefinedProject> {
        let parent = parent.into();
        let simple_file_path = |name: &str| YamlLang.find_build_script(&root_dir.join(name));
        match self {
            ProjectDefinition::Simple(s) => {
                let path =
                    simple_file_path(s).expect(&format!("no build script could be found at {s}"));
                vec![DefinedProject::new(s.to_string(), path, parent)]
            }
            ProjectDefinition::Adv {
                name,
                path,
                projects,
            } => {
                let mut output = vec![];
                let path = match path {
                    Some(path) => YamlLang.find_build_script(&root_dir.join(path)),
                    None => simple_file_path(name),
                }
                .expect(&format!(
                    "no build script could be found at {}",
                    path.as_ref()
                        .map(|p| format!("{p:?}"))
                        .unwrap_or(name.to_string())
                ));
                output.push(DefinedProject::new(name.to_string(), path.clone(), parent));
                if let Some(subs) = projects {
                    let new_root_dir = path.parent().unwrap();
                    output.extend(subs.iter().flat_map(|p| p.script_files(new_root_dir, name.to_string())));
                }
                output
            }
        }
    }
}
