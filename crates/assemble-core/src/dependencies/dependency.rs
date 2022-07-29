use std::collections::HashSet;
use crate::dependencies::self_resolving::ProjectDependency;
use crate::flow::attributes::ConfigurableAttributes;
use crate::flow::shared::ConfigurableArtifact;
use crate::Project;

/// A `Dependency` represents a dependency on the artifacts from a particular source.
pub trait Dependency : Clone {

    /// Set the reason why this dependency should be used
    fn because<S : AsRef<str>>(&mut self, reason: S);
    /// Check if two dependencies have identical values for their properties
    fn content_equals(&self, other: &Self) -> bool;

    /// Gets the identifier for this dependency
    fn module(&self) -> String;

    /// Gets the optional group for this dependency
    fn group(&self) -> Option<String>;

    /// Gets the version of the dependency
    fn version(&self) -> Option<String>;

    /// Gets the reason why this dependency should be used.
    fn reason(&self) -> &str;
}

/// A `ModuleDependency` is a [`Dependency`](Dependency) on a module outside the current project.
pub trait ModuleDependency : Dependency + ConfigurableAttributes
{

    fn artifacts(&self) -> HashSet<ConfigurableArtifact>;
    fn get_requested_features(&self) -> HashSet<String>;
    fn target_configuration(&self) -> String;

    fn requested_features<I : IntoIterator<Item=String>>(&mut self, features: I);
    fn set_target_configuration(&mut self, config: String);
}
