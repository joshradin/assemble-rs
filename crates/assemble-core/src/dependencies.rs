//! Describe mechanisms to enable projects to have dependencies

mod resolved_dependency;
mod dependency_type;
mod registry_container;
mod unresolved_dependency;

pub mod file_dependency;
pub mod dependency_container;
pub mod configurations;
pub mod artifact_request;

pub use dependency_type::*;
pub use resolved_dependency::*;
pub use registry_container::*;
pub use unresolved_dependency::*;
