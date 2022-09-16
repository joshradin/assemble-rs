//! The contents of the build file

use serde_yaml::Value;
use std::collections::HashMap;

/// The definitions required for a yaml build
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct YamlBuild {
    script: Option<Script>,
    #[serde(default)]
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    tasks: HashMap<String, TaskRequest>,
}

impl YamlBuild {
    pub fn script(&self) -> Option<&Script> {
        self.script.as_ref()
    }
    pub fn tasks(&self) -> &HashMap<String, TaskRequest> {
        &self.tasks
    }
}

/// Script settings
#[derive(Debug, Deserialize)]
pub struct Script {
    #[serde(rename = "plugins")]
    #[serde(default)]
    plugin_requests: Vec<PluginRequest>,
    #[serde(default)]
    #[serde(with = "serde_yaml::with::singleton_map_recursive")]
    repositories: Vec<RepositoryRequest>,
}

impl Script {
    pub fn plugin_requests(&self) -> &Vec<PluginRequest> {
        &self.plugin_requests
    }
    pub fn repositories(&self) -> &Vec<RepositoryRequest> {
        &self.repositories
    }
}

/// A plugin request
#[derive(Debug, Deserialize)]
pub struct PluginRequest {
    id: String,
    version: Option<String>,
    includes: Option<Vec<String>>,
}

impl PluginRequest {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }
    pub fn includes(&self) -> Option<&[String]> {
        self.includes.as_deref()
    }
}

/// A plugin request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepositoryRequest {
    Id(String),
    Url(String),
    Crates,
}

/// A task request
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskRequest {
    #[serde(default = "empty")]
    ty: String,
    depends_on: Option<Vec<String>>,
    vars: Option<HashMap<String, Value>>,
    #[serde(alias = "configure")]
    #[serde(alias = "lazy_evaluation")]
    props: Option<HashMap<String, Value>>,
}

impl TaskRequest {
    pub fn ty(&self) -> &str {
        &self.ty
    }
    pub fn depends_on(&self) -> Option<&Vec<String>> {
        self.depends_on.as_ref()
    }
    pub fn vars(&self) -> Option<&HashMap<String, Value>> {
        self.vars.as_ref()
    }

    pub fn props(&self) -> Option<&HashMap<String, Value>> {
        self.props.as_ref()
    }
}

fn empty() -> String {
    String::from("assemble_core::Empty")
}
