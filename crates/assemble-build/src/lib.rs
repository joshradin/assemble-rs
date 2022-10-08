//! Designed to be used as a build-dependency. Used to generate plugin descriptors

use std::path::PathBuf;

pub mod function_finder;
pub mod plugin_function;

#[derive(Debug)]
pub struct PluginError;

/// Creates plugin descriptor information by finding `#[plugin]` attributes
pub fn generate_plugin_metadata() -> Result<(), PluginError> {
    let lib_file = PathBuf::from_iter(&[
        &std::env::var("CARGO_MANIFEST_DIR").unwrap(),
        "src",
        "lib.rs",
    ]);

    println!("cargo:warning={:?}", lib_file);

    Ok(())
}
