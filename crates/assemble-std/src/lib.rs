//! # The Assemble Standard Library
//!
//! Contains extra stuff for assemble-daemon-rs that don't need to be in the core crate, but provide
//! good content.

#![deny(missing_docs)]
#![deny(broken_intra_doc_links)]

pub mod extensions;
pub mod specs;
pub mod tasks;

use assemble_core::{ExecutableTask, Project};
pub use crate::extensions::project_extensions::ProjectExec;
pub use crate::tasks::exec::Exec;
pub use crate::tasks::files::{Delete, Dupe};

pub use assemble_core::task::Empty;
use assemble_core::plug;

#[cfg(feature = "core")]
pub use assemble_core::Task;


/// Apply the standard plugin to this project.
#[plug(plugin_id = "assemble/std")]
pub fn std<E : ExecutableTask>(project: &mut Project<E>) {

}

mod private {
    use assemble_core::{ExecutableTask, Project};

    /// Trait can only be implemented in the assemble std library for the Project type.
    pub trait ProjectSealed {}

    impl<E : ExecutableTask> ProjectSealed for Project<E> {}
}
