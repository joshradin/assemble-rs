use std::path::Path;
use assemble_core::prelude::ProjectError;
use crate::build_logic::plugin::compilation::{CompiledScript, CompileLang};
use crate::build_logic::plugin::script::BuildScript;
use crate::build_logic::plugin::script::languages::YamlLang;

/// Compiles a yaml-based project
#[derive(Debug, Default)]
pub struct YamlCompiler;

impl CompileLang<YamlLang> for YamlCompiler {
    type Err = ProjectError;

    fn compile(script: &BuildScript<YamlLang>, output_path: &Path) -> Result<CompiledScript, Self::Err> {
        todo!()
    }
}


