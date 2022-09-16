//! Contains builders for making projects

use crate::build_logic::plugin::compilation::{CompiledScript, CompileLang};
use crate::build_logic::plugin::script::{BuildScript, ScriptingLang};
use assemble_core::__export::{CreateTask, InitializeTask, ProjectResult, TaskId, TaskIO};
use assemble_core::exception::{BuildError, BuildException};
use assemble_core::prelude::{Provider, SharedProject};
use assemble_core::lazy_evaluation::{Prop, VecProp};
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::{BuildResult, Executable, Project, Task};
use serde::{Serialize, Serializer};
use std::any::type_name;
use std::collections::HashMap;
use std::env::current_exe;
use std::error::Error;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::Write;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::time::Instant;
use toml_edit::{Document, value};

/// Simplified version of project lazy_evaluation
pub type ProjectProperties = HashMap<String, Option<String>>;

#[cfg(feature = "yaml")]
pub mod yaml;

mod create_cargo_file;
mod compile_project;

/// Define a builder to make projects. This trait is responsible for generating the `:build-logic`
/// project.
pub trait BuildSettings {
    /// The scripting language for this project
    type Lang: ScriptingLang;
    type Err: Error;

    /// Open a project in a specific directory
    fn open<P: AsRef<Path>>(
        &self,
        path: P,
        properties: &ProjectProperties,
    ) -> Result<SharedProject, Self::Err>;

    /// Attempt to find a project by searching up a directory
    fn discover<P: AsRef<Path>>(
        &self,
        path: P,
        properties: &ProjectProperties,
    ) -> Result<SharedProject, Self::Err>;
}

/// Compile a build script
#[derive(CreateTask, TaskIO)]
pub struct CompileBuildScript<S: ScriptingLang + Send + Sync, C: CompileLang<S> + Send + Sync> {
    #[input(file)]
    pub script_path: Prop<PathBuf>,
    #[input]
    scripting_lang: Prop<Lang<S>>,
    #[input]
    compile_lang: Prop<Lang<C>>,
    #[output]
    pub output_file: Prop<PathBuf>,
    compiled: Prop<CompiledScript>,
}

impl<S: ScriptingLang + Send + Sync, C: CompileLang<S> + Send + Sync> CompileBuildScript<S, C> {
    pub fn compiled_script(&self) -> impl Provider<CompiledScript> {
        self.compiled.clone()
    }
}

impl<S: ScriptingLang + Send + Sync, C: CompileLang<S> + Send + Sync> UpToDate
    for CompileBuildScript<S, C>
{
    // fn up_to_date(&self) -> bool {
    //     false
    // }
}

impl<S: ScriptingLang + Send + Sync, C: CompileLang<S> + Send + Sync> InitializeTask
    for CompileBuildScript<S, C>
{
    fn initialize(task: &mut Executable<Self>, _project: &Project) -> ProjectResult {
        task.scripting_lang.set(Lang::<S>::default())?;
        task.compile_lang.set(Lang::<C>::default())?;
        if let Ok(path) = current_exe() {
            task.work()
                .add_input_file("executable", move || path.clone())?;
        }
        Ok(())
    }
}

impl<S: ScriptingLang + Send + Sync, C: CompileLang<S> + Send + Sync> Debug
    for CompileBuildScript<S, C>
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompileBuildScript")
            .field("script_path", &self.script_path)
            .field("scripting_lang", &self.scripting_lang)
            .field("compile_lang", &self.compile_lang)
            .field("output_file", &self.output_file)
            .finish()
    }
}

impl<S: ScriptingLang + Send + Sync, C: CompileLang<S> + Send + Sync> Task
    for CompileBuildScript<S, C>
{
    fn task_action(task: &mut Executable<Self>, _project: &Project) -> BuildResult {
        let instant = Instant::now();
        let script_path = task.script_path.fallible_get()?;
        let ref build_script: BuildScript<S> = BuildScript::new(script_path);
        let output_path = task.output_file.fallible_get()?;
        let compiled = C::compile(build_script, &output_path).map_err(BuildException::new)?;
        task.compiled.set(compiled)?;
        info!("compiled in {:.3} sec", instant.elapsed().as_secs_f32());
        Ok(())
    }
}

pub struct Lang<S: Send>(PhantomData<S>);

impl<S: Send> Default for Lang<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<S: Send> Clone for Lang<S> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<L: Send> Serialize for Lang<L> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let type_name = type_name::<L>();
        type_name.serialize(serializer)
    }
}

impl<S: Send> Debug for Lang<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", type_name::<S>())
    }
}
