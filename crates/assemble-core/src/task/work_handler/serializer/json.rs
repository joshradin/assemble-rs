use crate::error::PayloadError;
use crate::prelude::ProjectError;
use crate::project::ProjectResult;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::str::FromStr;
use serde_json::Value;

/// Deserialize into a value
pub fn from_str<T: DeserializeOwned>(string: impl AsRef<str>) -> ProjectResult<T> {
    serde_json::from_str(string.as_ref()).map_err(|e| ProjectError::custom(e).into())
}

/// Serializes a value
pub fn to_string<S: Serialize>(value: &S) -> ProjectResult<String> {
    serde_json::to_string_pretty(value)
        .map_err(|e| ProjectError::custom(e).into())
}

/// Serializes a value
pub fn to_writer<W: Write, S: Serialize>(writer: W, value: &S) -> ProjectResult<()> {
    serde_json::to_writer_pretty(writer, value)
        .map_err(|e| ProjectError::custom(e).into())
}

/// A serializable value. Can *only* be serialized, but
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct Serializable(Value);

pub type SerializableError = serde_json::Error;

impl Serializable {
    /// Performs the conversion
    pub fn new<T: Serialize>(value: T) -> ProjectResult<Self> {
        let s = to_string(&value)?;
        Value::from_str(&s)
            .map(|v| Serializable(v))
            .map_err(|e| ProjectError::custom(e).into())
    }

    /// Turns this serializable into some value
    pub fn deserialize<T: DeserializeOwned>(&self) -> ProjectResult<T> {
        let string = serde_json::to_string(&self.0).map_err(|e| ProjectError::custom(e))?;
        from_str::<T>(string)
    }
}