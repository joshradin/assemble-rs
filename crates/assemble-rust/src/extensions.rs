//! Contains rust related extensions

use crate::toolchain::Toolchain;
use assemble_core::identifier::Id;
use assemble_core::properties::Prop;

/// The rust plugin extension
#[derive(Debug)]
pub struct RustPluginExtension {
    /// The default toolchain to use with the rust executables
    pub toolchain: Prop<Toolchain>,
}

impl RustPluginExtension {
    /// Creates a new instance of a rust extension
    pub fn new() -> Self {
        let mut extension = Self {
            toolchain: Prop::with_name("toolchain"),
        };
        extension.toolchain.set(Toolchain::stable()).unwrap();
        extension
    }
}
