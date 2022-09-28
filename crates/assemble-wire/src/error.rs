//! Defines error codes for interprocess packets

use crate::data::deserialize;
use crate::{FromData, IntoData};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::collections::VecDeque;
use std::convert::Infallible;

/// Some error occurred.
#[repr(u16)]
#[derive(Debug, thiserror::Error, FromPrimitive, Copy, Clone)]
pub enum Error {
    #[error("Server denied")]
    Denied = 1,
    #[error("given data was invalid")]
    Invalid,
}

impl FromData for Error {
    type Err = Infallible;

    fn deserialize(bytes: &mut VecDeque<u8>) -> Result<Self, Self::Err> {
        let u: u16 = deserialize(bytes)?;
        Ok(Error::from_u16(u).unwrap())
    }

    fn minimum_size() -> usize {
        2
    }
}

impl IntoData for Error {
    fn serialize(&self) -> Vec<u8> {
        (*self as u16).serialize()
    }
}
