//! Provides the serialization and deserialization functions used within this project.

use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::prelude::ProjectError;
use crate::project::ProjectResult;

/// Deserialize into a value
pub fn from_str<T : DeserializeOwned>(string: impl AsRef<str>) -> ProjectResult<T> {
    ron::from_str(string.as_ref()).map_err(|e| ProjectError::custom(e).into())
}

/// Serializes a value
pub fn to_string<S : Serialize>(value: &S) -> ProjectResult<String> {
    ron::to_string(value).map_err(|e| ProjectError::custom(e).into())
}