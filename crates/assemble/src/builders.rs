//! Contains builders for making projects

use parking_lot::RwLock;
use std::any::type_name;
use std::collections::HashMap;
use std::env::current_exe;
use std::error::Error;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use serde::{Serialize, Serializer};

use assemble_core::__export::ProjectResult;
use assemble_core::exception::BuildException;
use assemble_core::lazy_evaluation::Prop;
use assemble_core::prelude::{
    Assemble, ProjectId, Provider, Settings, SettingsAware, SharedProject,
};
use assemble_core::task::initialize_task::InitializeTask;
use assemble_core::task::up_to_date::UpToDate;
use assemble_core::{BuildResult, Executable, Project, Task};

use crate::build_logic::plugin::compilation::{CompileLang, CompiledScript};
use crate::build_logic::plugin::script::{BuildScript, ScriptingLang};

/// Simplified version of project lazy_evaluation
pub type ProjectProperties = HashMap<String, Option<String>>;

#[cfg(feature = "js")]
pub mod js;
#[cfg(feature = "yaml")]
pub mod yaml;

mod compile_project;
mod create_cargo_file;
mod create_lib_file;
mod patch_cargo;

/// Determines a builder type
pub const fn builder_type() -> &'static str {
    #[allow(unreachable_patterns)]
    match true {
        cfg!(feature = "yaml") => "yaml",
        cfg!(feature = "js") => "js",
        _ => "",
    }
}

/// Define a builder to make projects. This trait is responsible for generating the `:build-logic`
/// project.
pub trait BuildConfigurator {
    /// The scripting language for this project
    type Lang: ScriptingLang;
    type Err: Error;

    fn get_build_logic<S: SettingsAware>(&self, settings: &S) -> Result<BuildLogic, Self::Err>;

    fn configure_settings<S: SettingsAware>(&self, setting: &mut S) -> Result<(), Self::Err>;

    /// Attempt to find a project by searching up a directory. Creates a [`Settings`] instance.
    fn discover<P: AsRef<Path>>(
        &self,
        path: P,
        assemble: &Arc<RwLock<Assemble>>,
    ) -> Result<Settings, Self::Err>;
}

/// A build logic project
pub struct BuildLogic(SharedProject);

/// Compile a build script
#[derive(CreateTask, TaskIO)]
pub struct CompileBuildScript<S: ScriptingLang + Send + Sync, C: CompileLang<S> + Send + Sync> {
    pub project_id: Prop<ProjectId>,
    #[input(file)]
    pub script_path: Prop<PathBuf>,
    #[input]
    scripting_lang: Prop<Lang<S>>,
    #[input]
    compile_lang: Prop<Lang<C>>,
    #[output(file)]
    pub output_file: Prop<PathBuf>,
    #[output]
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
                .add_input_file("executable", provider!(move || path.clone()))?;
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
        let build_script: &BuildScript<S> =
            &BuildScript::new(script_path, task.project_id.fallible_get()?);
        let output_path = task.output_file.fallible_get()?;
        let compiled = C::compile(build_script, &output_path).map_err(BuildException::new)?;
        task.compiled.set(compiled)?;
        debug!("compiled in {:.3} sec", instant.elapsed().as_secs_f32());
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
