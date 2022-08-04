use crate::__export::TaskId;
use crate::file::RegularFile;
use crate::project::buildable::{Buildable, BuiltByContainer, IntoBuildable};
use crate::project::ProjectError;
use crate::workspace::Dir;
use crate::Project;
use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::path::{Path, PathBuf};
use time::{Date, OffsetDateTime};
use crate::dependencies::configurations::Configuration;

/// Represents the artifact output of some task
pub trait Artifact: Send + Sync {
    /// The classifier of the artifact, if any
    fn classifier(&self) -> Option<String>;
    /// The date that should be used when publishing the artifact.
    ///
    /// By default, nothing is returned
    fn date(&self) -> Option<Date> {
        None
    }
    /// The extension of this published artifact
    fn extension(&self) -> String;
    /// The name of the artifact
    fn name(&self) -> String;
    /// The type of the artifact.
    ///
    /// By default, this value is the name as the extension but can have a different value.
    fn artifact_type(&self) -> String;

    /// Gets the file for this artifact.
    ///
    /// By default, this value is `[name][-[classifier]].[extension]`
    fn file(&self) -> PathBuf {
        default_file(self)
    }

    fn buildable(&self) -> Option<Box<dyn Buildable>> {
        None
    }
}

fn default_file<A: Artifact + ?Sized>(artifact: &A) -> PathBuf {
    let as_string = format!(
        "{}{}.{}",
        artifact.name(),
        artifact
            .classifier()
            .map(|s| format!("-{}", s))
            .unwrap_or(String::new()),
        artifact.extension()
    );
    PathBuf::from(as_string)
}

/// A configurable artifact.
#[derive(Clone)]
pub struct ConfigurableArtifact {
    classifier: Option<String>,
    name: String,
    extension: String,
    artifact_type: Option<String>,
    built_by: BuiltByContainer,
    file: Option<PathBuf>,
}

impl ConfigurableArtifact {
    pub fn from_artifact<A: IntoArtifact>(artifact: A) -> Self
    where
        A::IntoArtifact: 'static,
    {
        let artifact = artifact.into_artifact();
        let mut container = BuiltByContainer::new();
        let mut output = Self {
            classifier: artifact.classifier(),
            name: artifact.name(),
            extension: artifact.extension(),
            artifact_type: Some(artifact.artifact_type()),
            built_by: container,
            file: Some(artifact.file()),
        };
        output
    }

    pub fn new(name: String, extension: String) -> Self {
        Self {
            classifier: None,
            name,
            extension,
            artifact_type: None,
            built_by: BuiltByContainer::new(),
            file: None,
        }
    }

    /// Set the name of the artifact
    pub fn set_name(&mut self, name: impl AsRef<str>) {
        self.name = name.as_ref().to_string();
    }

    /// Set the classifier of the artifact
    pub fn set_classifier(&mut self, classifier: impl AsRef<str>) {
        self.classifier = Some(classifier.as_ref().to_string());
    }
    /// Set the extension of the artifact
    pub fn set_extension(&mut self, extension: impl AsRef<str>) {
        self.extension = extension.as_ref().to_string();
    }

    /// Set the artifact's type
    pub fn set_artifact_type(&mut self, artifact_type: impl AsRef<str>) {
        self.artifact_type = Some(artifact_type.as_ref().to_string());
    }

    /// Set the file of the artifact
    pub fn set_file(&mut self, file: PathBuf) {
        self.file = Some(file);
    }

    /// Register some buildable that build this artifact
    pub fn built_by<B: IntoBuildable>(&mut self, build: B)
    where
        B::Buildable: 'static,
    {
        self.built_by.add(build)
    }
}

impl Buildable for ConfigurableArtifact {
    fn get_dependencies(&self, project: &Project) -> Result<HashSet<TaskId>, ProjectError> {
        self.built_by.get_dependencies(project)
    }
}

impl Debug for ConfigurableArtifact {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.file())
    }
}

impl Artifact for ConfigurableArtifact {
    fn classifier(&self) -> Option<String> {
        self.classifier.clone()
    }

    fn extension(&self) -> String {
        self.extension.clone()
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn artifact_type(&self) -> String {
        self.artifact_type.clone().unwrap_or(self.extension())
    }

    fn file(&self) -> PathBuf {
        self.file.clone().unwrap_or_else(|| default_file(self))
    }

    fn buildable(&self) -> Option<Box<dyn Buildable>> {
        Some(Box::new(self.built_by.clone()))
    }
}

/// Get access to some object's artifact
pub trait IntoArtifact {
    type IntoArtifact: Artifact;

    /// Get an artifact from some type
    fn into_artifact(self) -> Self::IntoArtifact;
}

impl<A: Artifact> IntoArtifact for A {
    type IntoArtifact = A;

    fn into_artifact(self) -> Self::IntoArtifact {
        self
    }
}

impl IntoArtifact for PathBuf {
    type IntoArtifact = ConfigurableArtifact;

    fn into_artifact(self) -> Self::IntoArtifact {
        self.as_path().into_artifact()
    }
}

impl IntoArtifact for &Path {
    type IntoArtifact = ConfigurableArtifact;

    fn into_artifact(self) -> Self::IntoArtifact {
        let name = self
            .file_name()
            .expect("no file name found")
            .to_str()
            .unwrap()
            .to_string();

        let mut artifact = if name.contains(".") {
            let name = name.rsplit_once(".").unwrap().0.to_string();
            let ext = self
                .extension()
                .expect("no extension found")
                .to_str()
                .unwrap()
                .to_string();
            ConfigurableArtifact::new(name, ext)
        } else {
            ConfigurableArtifact {
                classifier: None,
                name,
                extension: "".to_string(),
                artifact_type: Some("directory".to_string()),
                built_by: Default::default(),
                file: None
            }
        };
        artifact.set_file(self.to_path_buf());
        artifact
    }
}

impl IntoArtifact for RegularFile {
    type IntoArtifact = ConfigurableArtifact;

    fn into_artifact(self) -> Self::IntoArtifact {
        self.path().into_artifact()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ImmutableArtifact {
    classifier: Option<String>,
    name: String,
    extension: String,
    artifact_type: String,
    file: PathBuf,
}

impl ImmutableArtifact {
    pub fn new<A: IntoArtifact>(artifact: A) -> Self {
        let as_artifact = artifact.into_artifact();
        Self {
            classifier: as_artifact.classifier(),
            name: as_artifact.name(),
            extension: as_artifact.extension(),
            artifact_type: as_artifact.artifact_type(),
            file: as_artifact.file(),
        }
    }
}

impl Artifact for ImmutableArtifact {
    fn classifier(&self) -> Option<String> {
        self.classifier.clone()
    }

    fn extension(&self) -> String {
        self.extension.clone()
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn artifact_type(&self) -> String {
        self.artifact_type.clone()
    }

    fn file(&self) -> PathBuf {
        self.file.clone()
    }
}

#[cfg(test)]
mod tests {
    use crate::flow::shared::{Artifact, IntoArtifact};
    use std::path::PathBuf;

    #[test]
    fn artifact_from_path() {
        let path = PathBuf::from("artifact.zip");
        let artifact = path.into_artifact();
        assert_eq!(artifact.name(), "artifact");
        assert_eq!(artifact.extension(), "zip");
    }
}
