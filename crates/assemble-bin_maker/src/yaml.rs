use crate::declarations::{TaskDeclaration, TaskSettings};
use serde::de::{Error, MapAccess, Visitor};
use serde::Deserializer;
use serde_yaml::{Mapping, Value};
use std::collections::HashMap;
use std::fmt::Formatter;

/// Get a config yaml configuration for an assemble-daemon project
#[derive(Debug, Deserialize)]
pub struct AssembleYamlConfig {
    pub sources: Vec<String>,
    pub tasks: Vec<TaskDeclaration>,
}

#[cfg(test)]
mod tests {
    use crate::declarations::TaskDeclaration;

    #[test]
    fn get_task_declaration() {
        let task = r"
            hello_world:
              type: assemble-daemon::Exec
              configure:
                executable: 'echo'
                args: ['hello', 'world']

        ";

        let declaration: TaskDeclaration = serde_yaml::from_str(task).unwrap();
        println!("declaration: {:#?}", declaration);
        assert_eq!(declaration.identifier, "hello_world");
        assert_eq!(
            declaration.settings.r#type,
            Some("assemble-daemon::Exec".to_string())
        );
    }
}
