//! The startup_api part of assemble.
//!
//! Although the actual execution of assemble projects is handled by [`assemble-freight`][freight],
//! this module provides standard implementations for how the project will be initialized
//!
//! Project initialization is handled by two parts. The first is the [`Assemble`][assemble_struct]
//! instance, which provides a standard way of interpreting the start options for an assemble build.
//! Then, a [`Settings`][assemble_settings] instance is created that can be modified. The assemble-freight
//! project should provide the mechanisms that create these values.
//!
//!
//! [freight]: https://docs.rs/assemble-freight/latest/assemble_freight/
//! [assemble_struct]: crate::startup_api::invocation::Assemble;
//! [assemble_settings]: crate::startup_api::

pub mod execution_graph;
pub mod initialization;
pub mod invocation;
pub mod listeners;
