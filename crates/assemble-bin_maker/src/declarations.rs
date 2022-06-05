use serde::de::{EnumAccess, Error, MapAccess, SeqAccess, Visitor};
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
    pub r#type: Option<String>,
    pub configure: Option<HashMap<String, Value>>,
    pub first: Option<Actions>,
    pub last: Option<Actions>
}

#[derive(Debug)]
pub enum Actions {
    Single(String),
    List(Vec<String>)
}


struct ActionsVisitor;

impl<'de> Visitor<'de> for ActionsVisitor {
    type Value = Actions;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "Either a list or a single string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: Error {
        Ok(Actions::Single(v.to_string()))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error> where A: SeqAccess<'de> {
        let mut list = vec![];
        while let Some(entry) = seq.next_element::<String>()? {
            list.push(entry);
        }
        Ok(Actions::List(list))
    }
}

impl<'de> Deserialize<'de> for Actions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        deserializer.deserialize_any(ActionsVisitor)
    }
}

impl<'de> Deserialize<'de> for TaskDeclaration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(TaskDeclarationVisitor)
    }
}

struct TaskDeclarationVisitor;

impl<'de> Visitor<'de> for TaskDeclarationVisitor {
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
