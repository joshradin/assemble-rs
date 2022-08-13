//! Describe mechanisms to enable projects to have dependencies

mod dependency_type;
mod registry_container;
mod resolved_dependency;
mod unresolved_dependency;

pub mod artifact_request;
pub mod configurations;
pub mod dependency_container;
pub mod file_dependency;
pub mod project_dependency;

pub use dependency_type::*;
pub use registry_container::*;
pub use resolved_dependency::*;
pub use unresolved_dependency::*;
