//! External dependencies. These are dependencies that aren't built by this project in any way

use semver::VersionReq;
use crate::dependencies::{Dependency, ModuleDependency};
use crate::flow::attributes::{AttributeContainer, ConfigurableAttributes, HasAttributes};

/// A type that implements the external dependency
pub trait ExternalDependency : ModuleDependency {

    /// Some version constraint for this dependency
    fn version_constraint(&self) -> &VersionConstraint;
    /// Configure the version constraint for this dependency
    fn version<F : Fn(&mut VersionConstraint)>(&mut self, func: F);
}

pub trait ExternalModuleDependency : ExternalDependency {
    fn is_changing(&self) -> bool;
    fn set_changing(&mut self, changing: bool);
}

pub trait ClientModule : ExternalModuleDependency {

    /// Adds a dependency to the client module
    fn add_dependency<T : ModuleDependency>(&mut self, dependency: T);

    /// All the dependencies added to the client module
    fn dependencies(&self) -> Vec<()>;

    /// The id of the client module
    fn id(&self) -> String;
}

/// The specific version constraint for this
pub struct VersionConstraint {
    pub requirement: VersionReq,
    pub is_force: bool
}

pub struct ModuleComponentSelector {
    pub group: String,
    pub module: String,
    pub version_constraint: VersionConstraint,
    pub attributes: AttributeContainer
}

/// The main struct for defining external defined dependencies
pub struct ExtDependency {
    group: Option<String>,
    id: String,
    version_constraint: VersionConstraint,
    features: Vec<String>
}
