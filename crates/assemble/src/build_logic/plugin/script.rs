//! Create build scripts

use std::marker::PhantomData;
use std::path::PathBuf;

/// Marks a type as a scripting language
pub trait ScriptingLang {}

/// Languages the implement ScriptingLang by default
pub mod languages {
    use super::ScriptingLang;

    /// Configure a project using `yaml`
    #[cfg(feature = "yaml")]
    pub struct YamlLang;

    #[cfg(feature = "yaml")]
    impl ScriptingLang for YamlLang {}

    pub struct RustLang;
}

/// A build script
pub struct BuildScript<L: ScriptingLang> {
    lang: PhantomData<L>,
    contents: Vec<u8>,
}
