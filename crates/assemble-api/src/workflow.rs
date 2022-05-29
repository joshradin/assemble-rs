use crate::project::Project;

pub trait BinaryBuilder {
    type Error;

    fn build_binary(self, project: Project) -> Result<(), Self::Error>;
}
