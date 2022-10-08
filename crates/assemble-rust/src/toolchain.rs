//! Access to rust toolchains

use chrono::{Date, TimeZone, Utc};
use serde::Serializer;
use std::fmt::{Display, Formatter};

/// The toolchain channel
#[derive(Debug, Copy, Clone, Serialize)]
pub enum Channel {
    /// The stable channel
    Stable,
    /// The beta channel
    Beta,
    /// The nightly channel
    Nightly,
    /// A specific version of rust
    Version {
        /// The major version
        major: u32,
        /// The minor version
        minor: u32,
        /// An optional patch version. Most recent used if not specified
        patch: Option<u32>,
    },
}

impl Display for Channel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Channel::Stable => {
                write!(f, "stable")
            }
            Channel::Beta => {
                write!(f, "beta")
            }
            Channel::Nightly => {
                write!(f, "nightly")
            }
            Channel::Version {
                major,
                minor,
                patch,
            } => match patch {
                None => {
                    write!(f, "{}.{}", major, minor)
                }
                Some(patch) => {
                    write!(f, "{}.{}.{}", major, minor, patch)
                }
            },
        }
    }
}

/// A rust toolchain
#[derive(Debug, Serialize, Clone)]
pub struct Toolchain {
    /// The channel of the toolchain
    pub channel: Channel,
    /// An optional date of the toolchain
    #[serde(serialize_with = "serialize_date")]
    pub date: Option<Date<Utc>>,
    /// An optional target triple for the toolchain
    pub target_triple: Option<String>,
}

fn serialize_date<S>(date: &Option<Date<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match date {
        None => serializer.serialize_none(),
        Some(s) => {
            let date = s.format("%f").to_string();
            serializer.serialize_some(&date)
        }
    }
}

impl Toolchain {
    /// Create a new toolchain with a channel
    pub fn with_channel(channel: Channel) -> Self {
        Self {
            channel,
            date: None,
            target_triple: None,
        }
    }

    /// Create a new toolchain with a specific version
    pub fn with_version(major: u32, minor: u32) -> Self {
        Self {
            channel: Channel::Version {
                major,
                minor,
                patch: None,
            },
            date: None,
            target_triple: None,
        }
    }

    /// The stable toolchain
    pub fn stable() -> Self {
        Self::with_channel(Channel::Stable)
    }

    /// The nightly release channel
    pub fn nightly() -> Self {
        Self::with_channel(Channel::Nightly)
    }

    /// A nightly release on a specific date
    pub fn dated_nightly<Tz: TimeZone>(date: Date<Tz>) -> Self {
        let mut toolchain = Self::with_channel(Channel::Nightly);
        toolchain.date = Some(date.with_timezone(&Utc));
        toolchain
    }
}

impl Display for Toolchain {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.channel,
            self.date
                .as_ref()
                .map(|date| { format!("-{}", date.format("%F")) })
                .unwrap_or_default(),
            self.target_triple
                .as_ref()
                .map(|s| format!("-{}", s))
                .unwrap_or_default()
        )
    }
}
