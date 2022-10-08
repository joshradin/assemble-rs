use std::collections::HashMap;

#[derive(Debug, clap::Args, Clone, merge::Merge)]
pub struct ProjectProperties {
    /// Property flags
    #[clap(short = 'P', long = "project-property")]
    #[clap(value_parser(try_parse_property))]
    #[clap(value_name = "KEY[=VALUE]")]
    #[merge(strategy = merge::vec::append)]
    properties: Vec<(String, Option<String>)>,
}

const MISSING_VALUE: &str = "";

impl ProjectProperties {
    pub fn properties(&self) -> HashMap<String, Option<String>> {
        self.properties.clone().into_iter().collect()
    }

    /// Get a property
    pub fn property<S: AsRef<str>>(&self, prop: S) -> Option<&str> {
        let prop = prop.as_ref();
        for (key, value) in &self.properties {
            if key == prop {
                return Some(value.as_ref().map(|s| s.as_str()).unwrap_or(MISSING_VALUE));
            }
        }
        None
    }
}

fn try_parse_property(prop: &str) -> Result<(String, Option<String>), String> {
    if prop.contains('=') {
        let (prop, value) = prop.split_once('=').unwrap();
        Ok((prop.to_string(), Some(value.to_string())))
    } else {
        Ok((prop.to_string(), None))
    }
}
