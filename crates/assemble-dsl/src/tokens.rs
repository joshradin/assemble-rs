//! Token for the assemble-dsl
//!
//!
//!


mod ident;


use std::error::Error;
use std::fmt::{Debug, Display};
pub use ident::*;
use crate::source::Source;

use crate::span::Span;

/// A single token within an assemble program
pub trait Token : Debug {
    type Err : Error;

    /// Gets the span of this token
    fn span(&self) -> Span;

    /// Try to parse this token from some source
    fn parse(source: &Source, index: usize) -> Result<Self, Self::Err>;
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {

}