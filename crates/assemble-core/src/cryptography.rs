//! Cryptography functionality to aid with hashing and comparison

use generic_array::GenericArray;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::digest::{OutputSizeUser, Update};
use sha2::Digest;
use sha2::Sha256 as Sha2_Sha256;
use std::fmt::{Display, Formatter};
use std::io;
use std::num::ParseIntError;
use std::path::Path;
use std::str::FromStr;
use thiserror::Error;

type Sha256Length = <Sha2_Sha256 as OutputSizeUser>::OutputSize;

/// Number of bytes in the SHA-256 output
pub const SHA256_BYTES: usize = 256 / 8;

/// Output of sha256 hashing
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Sha256([u8; SHA256_BYTES]);
impl Sha256 {
    fn from(array: &GenericArray<u8, Sha256Length>) -> Self {
        let slice = array.as_slice();
        assert_eq!(slice.len(), SHA256_BYTES);
        let mut output_array = [0_u8; SHA256_BYTES];
        output_array.clone_from_slice(slice);
        Self(output_array)
    }
}

#[derive(Debug, Error)]
pub enum ParseSha256Error {
    #[error("Expected a string of 64 chars (len = {0})")]
    WrongSize(usize),
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
}

impl FromStr for Sha256 {
    type Err = ParseSha256Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.chars().count() != 64 {
            return Err(ParseSha256Error::WrongSize(s.chars().count()));
        }

        let mut bytes = [0_u8; SHA256_BYTES];
        for index in 0..SHA256_BYTES {
            let byte_str = &s[(index * 2)..][..2];
            let byte = u8::from_str_radix(byte_str, 16)?;
            bytes[index] = byte;
        }
        Ok(Self(bytes))
    }
}

impl Display for Sha256 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
        )
    }
}

impl Serialize for Sha256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Sha256 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        Sha256::from_str(&string).map_err(D::Error::custom)
    }
}

/// A hasher for creating Sha256 values
pub struct Sha256Hasher {
    hasher: Sha2_Sha256,
}

impl Sha256Hasher {
    /// Create a new hasher
    pub fn new() -> Self {
        Self {
            hasher: Sha2_Sha256::new(),
        }
    }

    /// Finalize the hasher
    pub fn finalize(self) -> Sha256 {
        Sha256::from(&self.hasher.finalize())
    }

    /// Update the hasher
    pub fn update<B: Sha256Hashable + ?Sized>(&mut self, value: &B) {
        value.hash(self);
    }
}

/// Types that can be hashed by a hasher
pub trait Sha256Hashable {
    fn hash(&self, hasher: &mut Sha256Hasher);
}

impl<B: AsRef<[u8]> + ?Sized> Sha256Hashable for B {
    fn hash(&self, hasher: &mut Sha256Hasher) {
        Digest::update(&mut hasher.hasher, self)
    }
}

/// Convenience method for hashing a value into a [`Sha256`](Sha256) value
pub fn hash_sha256<S: Sha256Hashable + ?Sized>(value: &S) -> Sha256 {
    let mut hasher = Sha256Hasher::new();
    hasher.update(value);
    hasher.finalize()
}

/// Convenience method for hashing a file into a [`Sha256`](Sha256) value
pub fn hash_file_sha256<P: AsRef<Path> + ?Sized>(value: &P) -> io::Result<Sha256> {
    let read = std::fs::read(value)?;
    Ok(hash_sha256(&read))
}

#[cfg(test)]
mod tests {
    use crate::cryptography::{hash_sha256, Sha256};
    use std::str::FromStr;

    #[test]
    fn parse_string() {
        let string = "design patterns is a really long and boring book";
        let value = hash_sha256(string);
        let value_as_string = value.to_string();
        let str_hash = Sha256::from_str(&value_as_string).unwrap();
        assert_eq!(value, str_hash, "value was parsed incorrectly");
    }

    #[test]
    fn hash_string() {
        let string1 = "Hello, World!";
        let string2 = "Hello, World!".to_string();
        let value1 = hash_sha256(string1);
        let value2 = hash_sha256(&string2);
        println!("{}", value1);
        assert_eq!(
            value1,
            Sha256::from_str("dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f")
                .unwrap()
        );
        assert_eq!(
            value1, value2,
            "Hashing of equivalent bytes should be equal"
        );
    }
}
