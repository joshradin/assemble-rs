//! Provides development tools, usually for testing purposes

use crate::error::PayloadError;
use crate::prelude::{ProjectId, SharedProject};
use crate::project::ProjectError;
use crate::{project, Project};
use serde::de::{Error, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fmt::Formatter;

#[derive(Debug)]
struct ProjectDesc {
    name: String,
    children: Vec<ProjectDesc>,
}

impl ProjectDesc {
    fn to_project(self) -> project::Result<SharedProject> {
        let id = ProjectId::new(&self.name)?;
        let project = Project::with_id(id)?;
        project.with_mut(|project| -> crate::project::Result<()> {
            for sub in self.children {
                sub.to_project_with_parent(project)?;
            }
            Ok(())
        })?;
        Ok(project)
    }

    fn to_project_with_parent(self, parent: &mut Project) -> project::Result<()> {
        parent.subproject(&self.name, |project| {
            for children in self.children {
                children.to_project_with_parent(project)?;
            }
            Ok(())
        })
    }
}

impl<'de> Deserialize<'de> for ProjectDesc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(ProjectDescVisitor)
    }
}

struct ProjectDescVisitor;
impl<'de> Visitor<'de> for ProjectDescVisitor {
    type Value = ProjectDesc;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "A map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let (name, children) = map
            .next_entry::<String, Vec<ProjectDesc>>()?
            .ok_or(A::Error::custom("one entry is required"))?;

        Ok(ProjectDesc { name, children })
    }
}

/// Quickly creates a project structure from yaml
pub fn quick_create(yaml: &str) -> project::Result<SharedProject> {
    let desc: ProjectDesc = serde_yaml::from_str(yaml).map_err(|e| ProjectError::custom(e))?;
    desc.to_project()
}

#[cfg(test)]
mod tests {
    use crate::project::dev::quick_create;
    use crate::project::GetProjectId;

    #[test]
    fn quick_create_test() {
        let project = quick_create(
            r"
        root_proj:
            - child1:
            - child2:
        ",
        )
        .expect("could not create project");
        assert_eq!(project.project_id(), ":root_proj");
        assert_eq!(
            project.get_subproject("child1").unwrap().project_id(),
            ":root_proj:child1"
        );
        assert_eq!(
            project.get_subproject("child2").unwrap().project_id(),
            ":root_proj:child2"
        );
    }

    #[test]
    fn no_multiple_keys() {
        quick_create(
            r"
        root1:
        root2:
        ",
        )
        .expect_err("should not be able to create a project here");
    }
}
