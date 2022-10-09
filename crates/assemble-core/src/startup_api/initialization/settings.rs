/// Declares the configuration required to instantiate and configure the hierarchy of [`SharedProject`](crate::project::SharedProject)
/// which are part of this build. There's exactly one settings instance that's created per
/// settings file.
///
/// # Assembling a mutli-project build
/// One of the purposes of the `Settings` object is to allow you to declare projects which are
/// included in this build.
///
/// When included, a [`ProjectDescriptor`][pd] is created which can be used to configure the default
/// values for several properties of the project.
///
/// [pd]: super::descriptor::ProjectDescriptor
///
/// # Using Settings in a Settings File
/// Depends on the builder..
///
pub struct Settings {


}