use serde::de::{Error, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_yaml::Value;
use std::collections::HashMap;
use std::fmt::Formatter;

#[derive(Debug)]
pub struct TaskDeclaration {
    pub identifier: String,
    pub settings: TaskSettings,
}

#[derive(Debug, Deserialize)]
pub struct TaskSettings {
    pub r#type: String,
    configure: HashMap<String, Value>,
}

impl<'de> Deserialize<'de> for TaskDeclaration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(TaskDeclarationVisiter)
    }
}

struct TaskDeclarationVisiter;

impl<'de> Visitor<'de> for TaskDeclarationVisiter {
    type Value = TaskDeclaration;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "A task declaration")
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let (key, value): (String, TaskSettings) = access
            .next_entry()?
            .ok_or(Error::missing_field("No settings"))?;

        Ok(TaskDeclaration {
            identifier: key,
            settings: value,
        })
    }
}
