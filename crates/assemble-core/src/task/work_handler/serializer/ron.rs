use crate::error::PayloadError;
use crate::prelude::ProjectError;
use crate::project::ProjectResult;
use ron_serde::ser::PrettyConfig;
use ron_serde::Value;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize, Serializer};
use std::io::Write;
use std::str::FromStr;

/// Deserialize into a value
pub fn from_str<T: DeserializeOwned>(string: impl AsRef<str>) -> ProjectResult<T> {
    ron_serde::from_str(string.as_ref()).map_err(|e| ProjectError::custom(e).into())
}

/// Serializes a value
pub fn to_string<S: Serialize>(value: &S) -> ProjectResult<String> {
    ron_serde::ser::to_string_pretty(value, PrettyConfig::new().struct_names(true))
        .map_err(|e| ProjectError::custom(e).into())
}

/// Serializes a value
pub fn to_writer<W: Write, S: Serialize>(writer: W, value: &S) -> ProjectResult<()> {
    ron_serde::ser::to_writer_pretty(writer, value, PrettyConfig::new().struct_names(true))
        .map_err(|e| ProjectError::custom(e).into())
}

/// A serializable value. Can *only* be serialized, but
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Serializable(Value);

pub type SerializableError = ron_serde::Error;

impl Serializable {
    /// Performs the conversion
    pub fn new<T: Serialize>(value: T) -> ProjectResult<Self> {
        Value::from_str(&to_string(&value)?)
            .map(Serializable)
            .map_err(|e| ProjectError::custom(e).into())
    }

    /// Turns this serializable into some value
    pub fn deserialize<T: DeserializeOwned>(&self) -> ProjectResult<T> {
        self.0.clone().into_rust()
            .map_err(|e| ProjectError::custom(e).into())
    }
}