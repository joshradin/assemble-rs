//! # The Assemble Standard Library
//!
//! Contains extra stuff for assemble-daemon-rs that don't need to be in the core crate, but provide
//! good content.

#![deny(missing_docs)]
#![deny(broken_intra_doc_links)]

pub mod extensions;
pub mod specs;
pub mod tasks;

pub use crate::extensions::project_extensions::ProjectExec;
pub use crate::tasks::exec::Exec;
pub use crate::tasks::files::{Delete, Dupe};
pub use crate::tasks::Empty;

mod private {
    use assemble_core::Project;

    /// Trait can only be implemented in the assemble-daemon std library for the Project type.
    pub trait ProjectSealed {}

    impl ProjectSealed for Project {}
}
