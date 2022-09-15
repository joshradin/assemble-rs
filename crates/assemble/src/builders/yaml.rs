use crate::build_logic::plugin::script::languages::YamlLang;
use crate::builders::{BuildSettings, ProjectProperties};
use assemble_core::prelude::{SharedProject, TaskId};
use assemble_core::Project;
use heck::ToLowerCamelCase;
use itertools::Itertools;
use settings::Settings;
use std::fs::File;
use std::path::{Path, PathBuf};
use assemble_core::error::PayloadError;
use assemble_core::project::error::ProjectError;

pub mod compiler;
pub mod settings;
pub mod yaml_build_file;
pub mod yaml_build_logic;

/// The name of the settings file to generate the initial `:build-logic` project from.
pub static SETTINGS_FILE_NAME: &str = "settings.assemble.yaml";
/// A property to control the name of the file to look for
pub static SETTINGS_PROPERTY: &str = "settings.file";

#[derive(Debug, Error)]
pub enum YamlBuilderError {
    #[error(transparent)]
    DeserializeError(#[from] serde_yaml::Error),
    #[error("No settings file could be found from path {0:?}")]
    MissingSettingsFile(PathBuf),
    #[error(transparent)]
    ProjectError(#[from] PayloadError<ProjectError>),
}
