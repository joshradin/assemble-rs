use std::fmt::{Display, Formatter};
use regex::Regex;

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub struct DependencyType {
    short_name: String,
    long_name: String,
    accepted_file_types: Vec<String>,
}

impl DependencyType {
    /// Create a new dependency type.
    pub fn new<'a>(
        short_name: &str,
        long_name: &str,
        accepted_file_types: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        Self {
            long_name: long_name.to_string(),
            short_name: short_name.to_string(),
            accepted_file_types: accepted_file_types
                .into_iter()
                .map(str::to_string)
                .collect(),
        }
    }

    /// Check if this dependency type supports a given file
    pub fn supports(&self, file_name: &str) -> bool {
        for pattern in &self.accepted_file_types {
            let glob = glob::Pattern::new(&pattern).expect("invalid glob pattern");
            if glob.matches(file_name) {
               return true;
            }
        }
        false
    }


    pub fn short_name(&self) -> &str {
        &self.short_name
    }
    pub fn long_name(&self) -> &str {
        &self.long_name
    }
}

impl Display for DependencyType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.short_name)
    }
}