use std::error::Error;
use std::path::{Path, PathBuf};
use crate::build_logic::plugin::script::{BuildScript, ScriptingLang};

/// Marks a type as a compiled language that can be compiled from a scripting lang
pub trait CompileLang<T : ScriptingLang> : Default + 'static {
    type Err : 'static + Error + Send + Sync;
    fn compile(script: &BuildScript<T>, output_path: &Path) -> Result<CompiledScript, Self::Err>;
}

/// The compiled script
#[derive(Clone)]
pub struct CompiledScript {
    filepath: PathBuf
}

pub enum CompiledScriptType {
    Project,
    FreightInput,
    CargoToml
}

impl CompiledScript {
    pub fn new<P : AsRef<Path>>(filepath: P) -> Self {
        Self { filepath: filepath.as_ref().to_path_buf() }
    }


    pub fn filepath(&self) -> &Path {
        &self.filepath
    }
}