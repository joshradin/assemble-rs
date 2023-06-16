//! Used for creating rust tasks in javascript


use std::any::Any;
use assemble_core::Task;
use assemble_core::task::FullTask;

pub struct RustTaskFactory {

}


trait CreateRustTask {
    fn create_task(&self) -> Box<dyn Any>;
}
