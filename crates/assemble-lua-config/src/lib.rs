use std::path::Path;
use assemble_core::Project;
use assemble_core::project::configuration::ConfigureProject;

pub mod settings;

pub struct LuaConfigurator {

}

impl ConfigureProject for LuaConfigurator {
    type Error = ();

    fn init<P: AsRef<Path>>(base_file: P) -> Result<Self, Self::Error> {
        Ok(Self { })
    }

    fn produce_project(self) -> Result<Project, Self::Error> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
