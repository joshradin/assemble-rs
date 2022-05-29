use crate::dependencies::Source;
use crate::task::Task;

#[derive(Debug)]
pub struct Project;

impl Project {
    pub fn tasks(&self) -> Vec<&dyn Task> {
        unimplemented!()
    }

    pub fn sources(&self) -> Vec<&dyn Source> {
        unimplemented!()
    }
}
