//! Contains "unstable" elements of assemble freight.
//!
//! All unstable features are always available in the core crate, and can be made available for downstream
//! crates by enabling specific feature flags.

pub mod text_factory;

/// The enabled features, configured via feature flags. All feature flags must also rely on
/// the `unstable` feature being enabled.
#[cfg(feature = "unstable")]
pub mod enabled {

    /// Unstable features to add to the `assemble-core` [`prelude`](crate::prelude)
    pub mod prelude {}

    #[feature(feature = "text_factory")]
    pub use super::text_factory;
}
