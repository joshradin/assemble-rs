//! Defines the traits that are used to configure [`Project`s](super::Project)
//!
//! Implementations for these should be probably be in their own crates.
//! The binmaker's only job should be to create binaries from configured projects.

use std::path::Path;
use crate::Project;
use super::ExecutableTask;

/// Types that are able to configure projects should implement this trait.
pub trait ConfigureProject<'de, T : ExecutableTask<'de>> : Sized {
    type Error;

    /// Initializes a project from some base file
    fn init<P : AsRef<Path>>(base_file: P) -> Result<Self, Self::Error>;

    /// Check if the current configuration of a project is up-to-date.
    ///
    /// By default, this always returns false.
    fn up_to_date(&self) -> bool { false }

    /// Compile sources, if necessary.
    ///
    /// By default, this does nothing. Useful if the language that is used to configure the project
    /// is compiled.
    ///
    /// > For example, a YAML file would not need to be compiled, but a rust one would probably need
    /// > to be compiled.
    fn compile_sources(&mut self) -> Result<(), Self::Error> { Ok(()) }

    /// Produces a configured project.
    fn produce_project(self) -> Result<Project<'de, T>, Self::Error>;

}