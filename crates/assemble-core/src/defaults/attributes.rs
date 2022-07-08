//! Default attributes for configuration matching

use crate::flow::attributes::Attribute;

/// The type of the dependency.
#[derive(Debug, Clone, PartialEq)]
pub struct Type;

impl Attribute for Type {}
