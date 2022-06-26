use assemble_core::task::DynamicTaskAction;
use assemble_core::{BuildResult, DefaultTask, Project};
use assemble_std::Task;

#[derive(Debug, Default, Task)]
pub struct Cargo {

}

impl DynamicTaskAction for Cargo {
    fn exec(&mut self, project: &Project) -> BuildResult {
        todo!()
    }
}
