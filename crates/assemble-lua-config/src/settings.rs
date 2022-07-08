use std::path::{Path, PathBuf};
use mlua::{Lua, UserData, UserDataFields, UserDataMethods};

#[derive(Clone, Debug)]
pub struct Settings {
    root_project: String,
    build_file: String
}

impl Settings {
    pub fn new(root_project: &str, build_file: &Path) -> Self {
        Self { root_project: root_project.to_string(), build_file: format!("{:?}", build_file) }
    }
}

impl UserData for Settings {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_set("root_project", |lua: &Lua, settings: &mut Settings, val: String| {
            settings.root_project = val;
            Ok(())
        })
    }

    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(_methods: &mut M) {

    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use mlua::Lua;
    use crate::settings::Settings;

    #[test]
    fn set_project_name() {
        let lua = Lua::new();
        let settings = Settings::new("test", Path::new("test"));
        lua.globals().set("settings", settings).unwrap();
        lua.load(r#"
        settings.root_project = "other_value"
        "#)
            .exec().unwrap();

        let settings = lua.globals().get::<_, Settings>("settings").unwrap();
        assert_eq!(settings.root_project, "other_value");
    }
}