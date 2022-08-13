//! External dependencies. These are dependencies that aren't built by this project in any way

use crate::dependencies::{Dependency, Error, ModuleDependency};
use crate::flow::attributes::{AttributeContainer, ConfigurableAttributes, HasAttributes};
use crate::flow::shared::ConfigurableArtifact;
use semver::VersionReq;
use std::collections::HashSet;
use std::str::FromStr;

/// A type that implements the external dependency
pub trait ExternalDependency: ModuleDependency {
    /// Some version constraint for this dependency
    fn version_constraint(&self) -> &VersionConstraint;
    /// Configure the version constraint for this dependency
    fn version<F: Fn(&mut VersionConstraint)>(&mut self, func: F);
}

pub trait ExternalModuleDependency: ExternalDependency {
    fn is_changing(&self) -> bool;
    fn set_changing(&mut self, changing: bool);
}

pub trait ClientModule: ExternalModuleDependency {
    /// Adds a dependency to the client module
    fn add_dependency<T: ModuleDependency>(&mut self, dependency: T);

    /// All the dependencies added to the client module
    fn dependencies(&self) -> Vec<&ExtDependency>;

    /// The id of the client module
    fn id(&self) -> String;
}

/// The specific version constraint for this
#[derive(Debug, Clone)]
pub struct VersionConstraint {
    pub requirement: VersionReq,
    pub is_force: bool,
}

impl FromStr for VersionConstraint {
    type Err = semver::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let version_req = VersionReq::parse(s)?;
        Ok(VersionConstraint {
            requirement: version_req,
            is_force: false,
        })
    }
}

pub struct ModuleComponentSelector {
    pub group: String,
    pub module: String,
    pub version_constraint: VersionConstraint,
    pub attributes: AttributeContainer,
}

/// The main struct for defining external defined dependencies
#[derive(Debug, Clone)]
pub struct ExtDependency {
    group: Option<String>,
    id: String,
    version_constraint: VersionConstraint,
    features: HashSet<String>,
    attributes: AttributeContainer,
    dependencies: Vec<ExtDependency>,
    changing: bool,
    configuration: String,
    reason: String,
}

impl ExtDependency {
    pub fn from_module<D: ModuleDependency>(module: D) -> Result<Self, Error> {
        Ok(Self {
            group: module.group(),
            id: module.module(),
            version_constraint: VersionConstraint::from_str(
                &module.version().ok_or(Error::MissingVersionSpecifier)?,
            )?,
            features: module.get_requested_features(),
            attributes: AttributeContainer,
            dependencies: module
                .artifacts()
                .into_iter()
                .map(|m| Self::from_module(m))
                .collect::<Result<_, _>>()?,
            changing: false,
            configuration: "".to_string(),
            reason: "".to_string(),
        })
    }
}

impl ExternalModuleDependency for ExtDependency {
    fn is_changing(&self) -> bool {
        self.changing
    }

    fn set_changing(&mut self, changing: bool) {
        self.changing = changing;
    }
}

impl ExternalDependency for ExtDependency {
    fn version_constraint(&self) -> &VersionConstraint {
        &self.version_constraint
    }

    fn version<F: Fn(&mut VersionConstraint)>(&mut self, func: F) {
        (func)(&mut self.version_constraint)
    }
}

impl ModuleDependency for ExtDependency {
    fn artifacts(&self) -> HashSet<ConfigurableArtifact> {
        HashSet::new()
    }

    fn get_requested_features(&self) -> HashSet<String> {
        self.features.clone()
    }

    fn target_configuration(&self) -> String {
        todo!()
    }

    fn requested_features<I: IntoIterator<Item = String>>(&mut self, features: I) {
        todo!()
    }

    fn set_target_configuration(&mut self, config: String) {
        todo!()
    }
}

impl Dependency for ExtDependency {
    fn because<S: AsRef<str>>(&mut self, reason: S) {
        self.reason = reason.as_ref().to_string();
    }

    fn content_equals(&self, other: &Self) -> bool {
        self.id == other.id && self.group == other.group && self.features == other.features
    }

    fn module(&self) -> String {
        self.id.clone()
    }

    fn group(&self) -> Option<String> {
        self.group.clone()
    }

    fn version(&self) -> Option<String> {
        None
    }

    fn reason(&self) -> &str {
        &self.reason
    }
}

impl ConfigurableAttributes for ExtDependency {
    fn attributes<F: FnOnce(&mut AttributeContainer)>(&mut self, func: F) {
        (func)(&mut self.attributes)
    }
}

impl HasAttributes for ExtDependency {
    fn get_attributes(&self) -> &AttributeContainer {
        &self.attributes
    }
}

impl ClientModule for ExtDependency {
    fn add_dependency<T: ModuleDependency>(&mut self, dependency: T) {
        self.dependencies
            .push(ExtDependency::from_module(dependency).unwrap());
    }

    fn dependencies(&self) -> Vec<&ExtDependency> {
        self.dependencies.iter().collect()
    }

    fn id(&self) -> String {
        match &self.group {
            None => self.id.clone(),
            Some(g) => {
                format!("{}:{}", g, self.id)
            }
        }
    }
}


