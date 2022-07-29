//! Repositories define a way of resolving external dependencies


use crate::dependencies::external::{ClientModule, ExternalDependency};
use crate::dependencies::self_resolving::FileCollectionDependency;
use crate::flow::attributes::ConfigurableAttributes;

pub trait Repository : ConfigurableAttributes {
    type Err;

    /// Try to download a dependency
    fn resolve<D : ClientModule>(&self, dependency: D) -> Result<FileCollectionDependency, Self::Err>;
}