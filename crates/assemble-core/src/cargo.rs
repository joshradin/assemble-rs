//! Provides a wrapper around cargo environment variables set during compilation.
//!
//! The available environment variables can be found [here](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates)

use heck::ToShoutySnakeCase;

use std::collections::HashMap;
use std::env::vars;
use std::path::PathBuf;

/// Provides access to cargo environment variables. Getting an instance of this struct
/// guarantees the existence of cargo related variables
#[derive(Debug)]
pub struct CargoEnv {
    vars: HashMap<String, String>,
}

impl CargoEnv {
    /// Get an entry by name. The name is automatically converted into upper snake case.
    ///
    /// If the key is not immediately available, `CARGO_` is appended to the front and that is also searched.
    pub fn get_entry<S: AsRef<str>>(&self, key: S) -> Option<&str> {
        let key = key.as_ref().to_shouty_snake_case();
        println!("key = {:?}", key);
        self.vars
            .get(&key)
            .or_else(|| self.vars.get(&format!("CARGO_{}", key)))
            .map(|s| s.as_str())
    }

    /// Gets the name of the current package
    pub fn package_name(&self) -> &str {
        self.get_entry("pkg-name")
            .expect("Should always be available")
    }

    /// Gets the manifest directory
    pub fn manifest_directory(&self) -> PathBuf {
        PathBuf::from(
            self.get_entry("manifest-dir")
                .expect("Should always be available"),
        )
    }
}

/// Gets the cargo environment if available
pub fn get_cargo_env() -> Option<CargoEnv> {
    let vars: HashMap<String, String> = HashMap::from_iter(vars());
    if vars.contains_key("CARGO") {
        Some(CargoEnv {
            vars: vars
                .into_iter()
                .filter(|(key, _)| key.starts_with("CARGO") || key == "OUT_DIR")
                .collect(),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::cargo::get_cargo_env;

    #[test]
    fn can_get_env() {
        let _ = get_cargo_env().expect("should be available because running tests through cargo");
    }

    #[test]
    fn can_get_pkg_name() {
        let cargo = get_cargo_env().unwrap();
        let name = cargo
            .get_entry("pkg-name")
            .expect("Failed because CARGO_ wasn't appended");
        assert_eq!(name, "assemble-core");
    }

    #[test]
    fn can_get_manifest_dir() {
        let cargo = get_cargo_env().unwrap();
        let dir = cargo.manifest_directory();
        assert!(dir.ends_with("assemble-core"));
    }
}
