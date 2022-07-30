//! Download dependencies from a simple server. Dependencies on such a server have a direct path
//! based on the artifact.

use std::path::PathBuf;
use url::Url;

/// A server registry. Should be able to accept both file dependencies and artifact dependencies
pub struct Server {
    url: Url,
    path_addition: Option<PathBuf>
}