use crate::task::property::TaskProperties;
use crate::IntoTask;

pub trait WriteIntoProperties: IntoTask + Sized {
    fn write_into_properties(self, properties: &mut TaskProperties) {
        self.set_properties(properties);
    }
}
