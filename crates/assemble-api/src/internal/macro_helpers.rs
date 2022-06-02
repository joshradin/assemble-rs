use crate::task::property::TaskProperties;

pub trait FromProperties {
    fn from_properties(properties: &mut TaskProperties) -> Self;
}

pub trait WriteIntoProperties {
    fn write_into_properties(self, properties: &mut TaskProperties);
}
