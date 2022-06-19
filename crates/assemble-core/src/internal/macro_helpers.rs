use crate::task::property::TaskProperties;
use crate::Task;

pub trait WriteIntoProperties: Task + Sized {
    fn write_into_properties(self, properties: &mut TaskProperties) {
        self.set_properties(properties);
    }
}
