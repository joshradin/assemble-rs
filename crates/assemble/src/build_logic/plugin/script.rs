//! Create build scripts

/// Marks a type as a scripting language
pub trait ScriptingLang { }

/// Languages the implement ScriptingLang by default
pub mod languages {
    use super::ScriptingLang;

    #[cfg(feature="yaml")]
    pub struct YamlLang;

    #[cfg(feature = "yaml")]
    impl ScriptingLang for YamlLang {}

    pub struct RustLang;
}