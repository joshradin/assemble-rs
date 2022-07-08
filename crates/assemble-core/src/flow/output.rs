//! The outputs of the assemble project

use std::collections::HashMap;
use std::sync::Arc;
use crate::flow::shared::{Artifact, ConfigurableArtifact, IntoArtifact};

/// The artifact handler
#[derive(Default)]
pub struct ArtifactHandler {
    variant_map: HashMap<String, Arc<dyn Artifact>>
}

impl ArtifactHandler {

    pub fn new() -> Self {
        Self {
            variant_map: Default::default()
        }
    }

    /// Adds an artifact for a configuration
    pub fn add<S, A>(&mut self, variant: S, artifact: A) -> impl Artifact
        where
            S : AsRef<str>,
            A : IntoArtifact,
            A::IntoArtifact : 'static
    {
        self.add_with(variant, artifact, |_| {})
    }

    pub fn add_with<S, A, F>(&mut self, configuration: S, artifact: A, config: F) -> impl Artifact
        where
            S : AsRef<str>,
            A : IntoArtifact,
            A::IntoArtifact : 'static,
            F : FnOnce(&mut ConfigurableArtifact)
    {
        let config_name = configuration.as_ref().to_string();
        let artifact = artifact.into_artifact();
        let mut configurable = ConfigurableArtifact::from_artifact(artifact);
        (config)(&mut configurable);
        self.variant_map.insert(config_name, Arc::new(configurable.clone()));
        configurable
    }

    pub(crate) fn get_artifact(&self, configuration: &str) -> Option<Arc<dyn Artifact>> {
        self.variant_map
            .get(configuration)
            .map(|b| b.clone())
    }
}