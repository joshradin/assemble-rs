use std::collections::{HashMap, HashSet};
use std::ops::{Add, AddAssign};
use std::path::PathBuf;
use crate::dependencies::Registry;
use crate::flow::shared::{Artifact, ConfigurableArtifact, ImmutableArtifact, IntoArtifact};
use crate::file_collection::FileSet;



/// A resolved dependency contains information on the artifacts it stores and the downloaded files
/// it refers to
#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    artifacts: HashSet<ImmutableArtifact>,
    files: HashSet<PathBuf>
}

impl ResolvedDependency {
    /// Gets the files that are associated with this resolved dependency
    pub fn artifact_files(&self) -> FileSet {
        self.files
            .iter()
            .fold(FileSet::new(), |fc, file| {
                fc + file
            })
    }

    /// Gets artifact that are associated with this resolved dependency
    pub fn artifacts(&self) -> HashSet<impl Artifact> {
        self.artifacts.clone()
    }

    /// Joins two resolved dependency into one
    pub fn join(self, other: Self) -> Self {
        Self {
            artifacts: self.artifacts.union(&other.artifacts).cloned().collect(),
            files: self.files.union(&other.files).cloned().collect()
        }
    }
}

pub struct ResolvedDependencyBuilder {
    artifacts: HashSet<ImmutableArtifact>
}

impl ResolvedDependencyBuilder {

    /// Ensures that there's always at least one artifact in the resolved dependency
    pub fn new<A : IntoArtifact>(artifact: A) -> Self {
        Self { artifacts: HashSet::from_iter([ImmutableArtifact::new(artifact)]) }
    }

    /// Add an object of type that can be turned into an artifact
    pub fn add<A : IntoArtifact>(&mut self, artifact: A) {
        self.artifacts.insert(ImmutableArtifact::new(artifact));
    }

    /// Add objects of type that can be turned into an artifact
    pub fn add_many<I, A : IntoArtifact>(&mut self, artifacts: I)
        where I : IntoIterator<Item=A>
    {
        for artifact in artifacts {
            self.add(artifact);
        }
    }

    pub fn finish(self) -> ResolvedDependency {
        let files = self.artifacts
            .iter()
            .map(|i| i.file())
            .collect::<HashSet<_>>();

        ResolvedDependency {
            artifacts: self.artifacts,
            files
        }
    }
}

impl <A : IntoArtifact> AddAssign<A> for ResolvedDependencyBuilder {
    fn add_assign(&mut self, rhs: A) {
        self.add(rhs)
    }
}