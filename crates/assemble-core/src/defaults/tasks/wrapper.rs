//! The wrapper task allows for creating a wrapper for assemble that should never fail

use crate::__export::{CreateTask, InitializeTask, TaskIO, TaskId};
use crate::cryptography::Sha256;
use crate::defaults::tasks::wrapper::github::GetDistribution;
use crate::exception::BuildException;
use crate::lazy_evaluation::{Prop, Provider, ProviderExt};
use crate::project::error::ProjectError;
use crate::project::error::ProjectResult;
use crate::task::flags::{OptionDeclarationBuilder, OptionDeclarations, OptionsDecoder};
use crate::task::up_to_date::UpToDate;
use crate::workspace::WorkspaceDirectory;
use crate::{cryptography, provider, BuildResult, Executable, Project, Task, ASSEMBLE_HOME};
use serde_json::to_writer_pretty;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use strum_macros::{Display, EnumIter};
use toml::toml;
use toml_edit::{value, Document};
use url::Url;

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
    fn shell_script_location(&self) -> impl Provider<PathBuf> {
        let workspace = self.project().with(|p| p.root_dir());
        self.wrapper_name
            .clone()
            .map(move |name| workspace.join(name))
    }

    fn bat_script_location(&self) -> impl Provider<PathBuf> {
        let workspace = self.project().with(|p| p.root_dir());
        self.wrapper_name
            .clone()
            .map(move |name| workspace.join(format!("{}.bat", name)))
    }

    fn get_release_url(&self) -> Result<Url, ProjectError> {
        let distribution = github::get_distributions(&self.assemble_version.get())?.get_relevant();
        self.assemble_url
            .try_get()
            .or_else(|| distribution.map(|d| d.url))
            .ok_or_else(|| ProjectError::custom("No distribution could be determined"))
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
            OptionDeclarationBuilder::<String>::new("url")
                .use_from_str()
                .optional(true)
                .build(),
        ]))
    }

    fn try_set_from_decoder(&mut self, decoder: &OptionsDecoder) -> ProjectResult<()> {
        if let Some(version) = decoder.get_value::<String>("version")? {
            self.assemble_version
                .set(version)?;
        }
        if let Some(url) = decoder.get_value::<String>("url")? {
           let url = Url::parse(&url).map_err(|e| ProjectError::custom(e))?;
            self.assemble_url.set(url)?;
        }


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

        let mut settings: Document = {
            let mut file = File::open(&wrapper_properties_path)?;
            let mut toml = Vec::new();
            file.read_to_end(&mut toml)?;
            let as_string = String::from_utf8(toml)?;
            as_string.parse().map_err(|e| BuildException::new(e))?
        };

        let mut updated_url = false;
        let distribution_url = task.assemble_url.fallible_get()?;
        if settings["url"].to_string() != distribution_url.to_string() {
            settings["url"] = value(distribution_url.to_string());
            updated_url = true;
        }

        let wrapper_settings = toml_edit::de::from_document::<WrapperSettings>(settings.clone())?;

        if let Some(distribution_info) = wrapper_settings.existing_distribution() {
            if distribution_info.is_valid() && !updated_url {
                task.work().set_did_work(false);
                return Err(BuildException::StopTask.into());
            }
        }

        info!("settings = {:#?}", settings);

        let shell_file = task.shell_script_location().fallible_get()?;
        let bat_file = task.bat_script_location().fallible_get()?;

        {
            let mut file = File::create(&wrapper_properties_path)?;
            writeln!(file, "{}", settings.to_string())?;
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
        println!("replaced = {path:?}");
        Path::new(&path).join(&self.dist_path.trim_start_matches("/"))
    }

    fn store_path(&self) -> PathBuf {
        let path = self
            .store_base
            .as_ref()
            .unwrap_or(&self.dist_base)
            .replace("ASSEMBLE_HOME", &*ASSEMBLE_HOME.path().to_string_lossy());
        println!("replaced = {path:?}");
        Path::new(&path)
            .join(
                self.store_path
                    .as_ref()
                    .unwrap_or(&self.dist_path)
                    .trim_start_matches("/"),
            )
            .join(
                PathBuf::from(self.url.path())
                    .file_name()
                    .expect("no file path"),
            )
    }

    fn config_path(&self) -> PathBuf {
        self.dist_path().join("config.json")
    }

    fn existing_distribution(&self) -> Option<DistributionInfo> {
        let path = self.config_path();
        let file = File::open(path).ok()?;

        serde_json::from_reader(file).ok()
    }

    fn save_distribution_info(&self, info: DistributionInfo) -> io::Result<()> {
        let path = self.config_path();
        let file = File::create(path)?;

        serde_json::to_writer_pretty(file, &info)?;
        Ok(())
    }
}

/// Downloaded distribution info.
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct DistributionInfo {
    distribution: Distribution,
    executable_path: PathBuf,
    sha256: Sha256,
}

impl DistributionInfo {
    pub fn executable_path(&self) -> &PathBuf {
        &self.executable_path
    }

    /// Check whether the distribution is valid
    pub fn is_valid(&self) -> bool {
        if !self.executable_path.exists() {
            return false;
        }
        let metadata = self
            .executable_path
            .metadata()
            .expect("could not get metadata");
        if !metadata.is_file() {
            return false;
        }

        let sha = cryptography::hash_file_sha256(&self.executable_path)
            .expect("could not get sha256 of file");
        sha != self.sha256
    }
}

/// A distribution of assemble
#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Distribution {
    /// The url the distribution can be downloaded from
    pub url: Url,
    /// The os this distribution supports
    pub os: Os,
}

/// The os of a host system
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, EnumIter, Display, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
pub enum Os {
    #[cfg_attr(target_os = "macos", default)]
    MacOs,
    #[cfg_attr(target_os = "windows", default)]
    Windows,
    #[cfg_attr(target_os = "linux", default)]
    Linux,
}

#[cfg(test)]
mod tests {
    use crate::defaults::tasks::wrapper::WrapperSettings;
    use serde_json::json;
    use toml::toml;

    #[test]
    fn get_distribution_info_from_settings() {
        let settings = toml! {
            url = "https://github.com/joshradin/assemble-rs/releases/download/v0.1.2/assemble-linux-amd64"
            dist_base = "ASSEMBLE_HOME"
            dist_path = "/wrapper/dists"

        }.try_into::<WrapperSettings>().unwrap();

        println!("dist_base = {:?}", settings.dist_base);
        println!("dist_path = {:?}", settings.dist_path());
        println!("store_path = {:?}", settings.store_path());
    }
}
