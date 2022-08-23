use crate::build_logic::plugin::compilation::{CompiledScript, CompileLang};
use crate::build_logic::plugin::script::BuildScript;
use crate::build_logic::plugin::script::languages::YamlLang;

/// Compiles a yaml-based project
pub struct YamlCompiler;

impl CompileLang<YamlLang> for YamlCompiler {
    type Err = ();

    fn compile(&self, script: &BuildScript<YamlLang>) -> Result<CompiledScript, Self::Err> {
        todo!()
    }
}


