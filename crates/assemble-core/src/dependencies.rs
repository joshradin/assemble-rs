//! Describes external and internal dependencies. All dependencies should have two states. Unresolved
//! and resolved. Resolved dependencies should be marked with names starting with Resolved.

mod dependency;
pub use dependency::*;
pub mod self_resolving;
pub mod external;
pub mod repository;

pub mod sources;

#[derive(Debug, thiserror::Error)]
pub enum Error {

}