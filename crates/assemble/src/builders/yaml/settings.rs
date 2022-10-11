//! The settings configuration

use parking_lot::RwLock;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::build_logic::plugin::script::languages::YamlLang;
use crate::build_logic::plugin::script::ScriptingLang;
use assemble_core::identifier::Id;
use assemble_core::prelude::{
    Assemble, AssembleAware, ProjectBuilder, ProjectId, ProjectResult, Settings, SettingsAware,
};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YamlSettings {
    name: String,
    /// Project definitions
    #[serde(default)]
    projects: Vec<ProjectDefinition>,
}

impl YamlSettings {
    /// The name of the root project
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets the build script files associated with this settings. The first path is the root project
    pub fn projects(&self, root_dir: &Path) -> Vec<DefinedProject> {
        let mut output: Vec<_> = self
            .projects
            .iter()
            .flat_map(|s| s.script_files(root_dir, self.name.to_string()))
            .collect();
        if let Some(script) = YamlLang.find_build_script(root_dir) {
            output.insert(0, DefinedProject::new(self.name.clone(), script, None));
        }
        output
    }

    pub fn configure_settings<S: SettingsAware>(self, settings: &mut S) {
        settings.with_settings_mut(|settings| {
            settings.root_project_mut().set_name(&self.name);

            let mut project_stack = VecDeque::new();
            project_stack.extend(self.projects);

            let root_dir = settings.with_settings(|s| s.root_dir().to_path_buf());

            while let Some(project_def) = project_stack.pop_front() {
                match &project_def {
                    ProjectDefinition::Simple(s) => {
                        settings.include(s);
                    }
                    ProjectDefinition::Adv { name, .. } => {
                        settings.add_project(name, |b| {
                            project_def.add_to_settings(b, root_dir.clone())
                        });
                    }
                }
            }
        })
    }
}

/// A located project represents a project with a name and a path
#[derive(Debug)]
pub struct DefinedProject {
    name: String,
    path: PathBuf,
    parent: Option<String>,
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

    pub fn project_id(&self) -> ProjectResult<ProjectId> {
        let id = ProjectId::new(&self.name)?;
        if let Some(parent) = &self.parent {
            let parent = ProjectId::new(parent)?;
            Ok(Id::from(parent).concat(id.into()).into())
        } else {
            Ok(id)
        }
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
    fn add_to_settings(&self, builder: &mut ProjectBuilder, parent_dir: PathBuf) {
        match self {
            ProjectDefinition::Simple(s) => {
                builder.set_name(s);
                builder.set_dir(parent_dir.join(s));
            }
            ProjectDefinition::Adv {
                name,
                path,
                projects,
            } => {
                builder.set_name(name);
                let project_dir = if let Some(dir) = path {
                    parent_dir.join(dir)
                } else {
                    parent_dir.join(name)
                };
                builder.set_dir(&project_dir);
                if let Some(subs) = projects {
                    for sub in subs {
                        builder.project("", |s| sub.add_to_settings(s, project_dir.clone()))
                    }
                }
            }
        }
    }

    /// Gets the build script files associated with this settings
    fn script_files(
        &self,
        root_dir: &Path,
        parent: impl Into<Option<String>>,
    ) -> Vec<DefinedProject> {
        let parent = parent.into();
        let simple_file_path = |name: &str| YamlLang.find_build_script(&root_dir.join(name));
        match self {
            ProjectDefinition::Simple(s) => {
                let path = simple_file_path(s)
                    .unwrap_or_else(|| panic!("no build script could be found at {s}"));
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
                .unwrap_or_else(|| {
                    panic!(
                        "no build script could be found at {}",
                        path.as_ref()
                            .map(|p| format!("{p:?}"))
                            .unwrap_or(name.to_string())
                    )
                });
                output.push(DefinedProject::new(name.to_string(), path.clone(), parent));
                if let Some(subs) = projects {
                    let new_root_dir = path.parent().unwrap();
                    output.extend(
                        subs.iter()
                            .flat_map(|p| p.script_files(new_root_dir, name.to_string())),
                    );
                }
                output
            }
        }
    }
}
