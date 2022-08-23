//! Run cargo commands

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::Index;
use std::path::Path;

pub mod build;
pub mod publish;

/// The target for a cargo command. This can either be packages, the whole workspace, the lib, tests, bins,
/// or examples
#[derive(Debug, Clone, Serialize)]
pub enum Target {
    /// A package target
    Package(String),
    /// Target the entire workspace
    Workspace,
    /// Target only this package's library
    Lib,
    /// Target a specific binary
    Bin(String),
    /// Target all the binaries in the crate
    Bins,
    /// Target a test
    Test(String),
    /// Target all tests in the crate
    Tests,
    /// Target an example in the crate
    Example(String),
    /// Target all examples in the crate
    Examples,
    /// Targets all targets (? what does this mean ?)
    AllTarget
}
