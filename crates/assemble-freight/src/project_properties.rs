use std::collections::HashMap;

#[derive(Debug, clap::Args)]
pub struct ProjectProperties {
    /// Property flags
    #[clap(short = 'P', long = "project-property")]
    #[clap(value_parser(try_parse_property))]
    properties: Vec<(String, Option<String>)>,
}

impl ProjectProperties {
    pub fn properties(&self) -> HashMap<String, Option<String>> {
        self.properties.clone().into_iter().collect()
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
