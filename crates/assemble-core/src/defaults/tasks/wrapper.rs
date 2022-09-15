//! The wrapper task allows for creating a wrapper for assemble that should never fail

use crate::__export::{CreateTask, InitializeTask, TaskId, TaskIO};
use crate::cryptography::Sha256;
use crate::exception::BuildException;
use crate::project::error::ProjectError;
use crate::properties::{Prop, Provides, ProvidesExt};
use crate::task::flags::{OptionDeclarationBuilder, OptionDeclarations, OptionsDecoder};
use crate::task::up_to_date::UpToDate;
use crate::workspace::WorkspaceDirectory;
use crate::{ASSEMBLE_HOME, BuildResult, Executable, Project, Task};
use serde_json::to_writer_pretty;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::Read;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use toml::toml;
use url::Url;
use crate::project::error::ProjectResult;

mod github;

/// Create assemble wrapper files
#[derive(Debug)]
pub struct WrapperTask {
    /// The base name of the generate wrapper file. Appended with .bat for batch file variant
    pub wrapper_name: Prop<String>,
    /// The url of the specified assemble distributable
    pub assemble_url: Prop<Url>,
    /// if a direct url isn't provided, download from default provider with given version
    pub assemble_version: Prop<String>,
    /// If provided, compare the downloaded file with a string representing it's sha256 value. Fails
    /// if downloaded file doesn't match
    pub assemble_sha256: Prop<Sha256>,
}

impl Executable<WrapperTask> {
    fn shell_script_location(&self) -> impl Provides<PathBuf> {
        let workspace = self.project().with(|p| p.root_dir());
        self.wrapper_name
            .clone()
            .map(move |name| workspace.join(name))
    }

    fn bat_script_location(&self) -> impl Provides<PathBuf> {
        let workspace = self.project().with(|p| p.root_dir());
        self.wrapper_name
            .clone()
            .map(move |name| workspace.join(format!("{}.bat", name)))
    }

    fn get_release_url(&self) -> Result<Url, ProjectError> {
        Ok(self
            .assemble_url
            .try_get()
            .unwrap_or(github::get_distribution_url(&self.assemble_version.get())?))
    }
}

impl UpToDate for WrapperTask {}

impl CreateTask for WrapperTask {
    fn new(using_id: &TaskId, _project: &Project) -> ProjectResult<Self> {
        Ok(Self {
            wrapper_name: using_id.prop("name")?,
            assemble_url: using_id.prop("url")?,
            assemble_version: using_id.prop("version")?,
            assemble_sha256: using_id.prop("sha256")?,
        })
    }

    fn description() -> String {
        "Creates wrapper files for running assemble without requiring assemble to be already installed".to_string()
    }

    fn options_declarations() -> Option<OptionDeclarations> {
        Some(OptionDeclarations::new::<Self, _>([
            OptionDeclarationBuilder::<String>::new("version")
                .use_from_str()
                .optional(true)
                .build(),
        ]))
    }

    fn try_set_from_decoder(&mut self, decoder: &OptionsDecoder) -> ProjectResult<()> {
        self.assemble_version
            .set_with(decoder.get_value::<String>("version")?)?;
        Ok(())
    }
}

impl InitializeTask for WrapperTask {
    fn initialize(task: &mut Executable<Self>, _project: &Project) -> ProjectResult {
        let default_version = env!("CARGO_PKG_VERSION");
        task.assemble_version.set(default_version)?;
        task.wrapper_name.set("assemble")?;
        Ok(())
    }
}

impl TaskIO for WrapperTask {
    fn configure_io(task: &mut Executable<Self>) -> ProjectResult {
        let shell_script = task.shell_script_location();
        let bat_script = task.bat_script_location();
        task.work().add_output_provider(shell_script);
        task.work().add_output_provider(bat_script);
        Ok(())
    }
}

impl Task for WrapperTask {
    fn task_action(task: &mut Executable<Self>, project: &Project) -> BuildResult {
        let wrapper_properties_path = project
            .root_dir()
            .join("assemble")
            .join("wrapper")
            .join("assemble.toml");

        let mut settings: WrapperSettings = {
            let mut file = File::open(&wrapper_properties_path)?;
            let mut toml = Vec::new();
            file.read_to_end(&mut toml)?;
            toml::from_slice(&toml).map_err(|e| BuildException::new(e))?
        };

        info!("settings = {:#?}", settings);

        let shell_file = task.shell_script_location().fallible_get()?;
        let bat_file = task.bat_script_location().fallible_get()?;

        {
            let settings = toml::to_string_pretty(&settings).map_err(|e| BuildException::new(e))?;
            let mut file = File::create(&wrapper_properties_path)?;
            writeln!(file, "{}", settings)?;
        }

        Ok(())
    }
}

fn generate_shell_script(dest_file: &Path) -> Result<(), BuildResult> {
    Ok(())
}

fn generate_bat_script(dest_file: &Path) -> Result<(), BuildResult> {
    Ok(())
}

#[derive(Debug, Deserialize, Serialize)]
struct WrapperSettings {
    url: Url,
    sha256: Option<Sha256>,
    dist_base: String,
    store_base: Option<String>,
    dist_path: String,
    store_path: Option<String>,
}

impl WrapperSettings {
    fn dist_path(&self) -> PathBuf {
        let path = self
            .dist_base
            .replace("ASSEMBLE_HOME", &*ASSEMBLE_HOME.path().to_string_lossy());
        Path::new(&path).join(&self.dist_path)
    }

    fn store_path(&self) -> PathBuf {
        let path = self
            .store_base
            .as_ref()
            .unwrap_or(&self.dist_base)
            .replace("ASSEMBLE_HOME", &*ASSEMBLE_HOME.path().to_string_lossy());
        Path::new(&path).join(self.store_path.as_ref().unwrap_or(&self.dist_path))
    }
}
