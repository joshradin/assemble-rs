use crate::error::PayloadError;
use crate::prelude::ProjectError;
use crate::project::ProjectResult;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::str::FromStr;

/// Deserialize into a value
pub fn from_str<T: DeserializeOwned>(string: impl AsRef<str>) -> ProjectResult<T> {
    rmp_serde::from_slice(&string.as_ref().bytes().collect::<Vec<_>>())
        .map_err(|e| ProjectError::custom(e).into())
}

/// Serializes a value
pub fn to_string<S: Serialize>(value: &S) -> ProjectResult<String> {
    rmp_serde::to_vec(&value)
        .map(|s| String::from_utf8_lossy(&s).to_string())
        .map_err(|e| ProjectError::custom(e).into())
}

/// Serializes a value
pub fn to_writer<W: Write, S: Serialize>(mut writer: W, value: &S) -> ProjectResult<()> {
    let string = to_string(value)?;
    write!(writer, "{}", string)?;
    writer.flush()?;
    Ok(())
}

/// A serializable value. Can *only* be serialized, but
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(transparent)]
pub struct Serializable(Vec<u8>);

pub type SerializableError = serde_json::Error;

impl Serializable {
    /// Performs the conversion
    pub fn new<T: Serialize>(value: T) -> ProjectResult<Self> {
        let to_vec = rmp_serde::to_vec(&value).map_err(|e| ProjectError::custom(e))?;
        Ok(Self(to_vec))
    }

    /// Turns this serializable into some value
    pub fn deserialize<T: DeserializeOwned>(&self) -> ProjectResult<T> {
        rmp_serde::from_slice(&self.0).map_err(|e| ProjectError::custom(e).into())
    }
}
