use crate::builders::{Builder, ProjectProperties};
use assemble_core::prelude::SharedProject;
use std::path::Path;

pub struct YamlBuilder;

impl Builder for YamlBuilder {
    type Err = YamlBuilderError;

    fn open<P: AsRef<Path>>(
        path: P,
        properties: &ProjectProperties,
    ) -> Result<SharedProject, Self::Err> {
        todo!()
    }

    fn discover<P: AsRef<Path>>(
        path: P,
        properties: &ProjectProperties,
    ) -> Result<SharedProject, Self::Err> {
        todo!()
    }
}

#[derive(Debug, Error)]
pub enum YamlBuilderError {
    #[error(transparent)]
    DeserializeError(#[from] serde_yaml::Error),
}
