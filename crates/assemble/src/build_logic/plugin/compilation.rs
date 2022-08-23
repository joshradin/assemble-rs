use crate::build_logic::plugin::script::{BuildScript, ScriptingLang};
use std::error::Error;
use std::path::{Path, PathBuf};

/// Marks a type as a compiled language that can be compiled from a scripting lang
pub trait CompileLang<T: ScriptingLang>: Default + 'static {
    type Err: 'static + Error + Send + Sync;
    fn compile(script: &BuildScript<T>, output_path: &Path) -> Result<CompiledScript, Self::Err>;
}

/// The compiled script
#[derive(Clone)]
pub struct CompiledScript {
    filepath: PathBuf,
    dependencies: Vec<(String, String)>,
}

pub enum CompiledScriptType {
    Project,
    FreightInput,
    CargoToml,
}

impl CompiledScript {
    pub fn new<P: AsRef<Path>, I: IntoIterator<Item = (String, String)>>(
        filepath: P,
        dependencies: I,
    ) -> Self {
        Self {
            filepath: filepath.as_ref().to_path_buf(),
            dependencies: dependencies.into_iter().collect(),
        }
    }

    pub fn filepath(&self) -> &Path {
        &self.filepath
    }
}
