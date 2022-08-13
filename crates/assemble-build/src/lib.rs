//! Designed to be used as a build-dependency. Used to generate plugin descriptors

use crate::function_finder::FunctionFinder;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

pub mod function_finder;
pub mod plugin_function;

/// Creates plugin descriptor information by finding `#[plugin]` attributes
pub fn generate_plugin_metadata() -> Result<(), ()> {

    let lib_file = PathBuf::from_iter(&[
        &std::env::var("CARGO_MANIFEST_DIR").unwrap(),
        "src",
        "lib.rs",
    ]);


    // let function_finder = FunctionFinder::find_all(&lib_file);
    // let functions = function_finder.functions()
    //     .map(|item_fn| &item_fn.sig.ident)
    //     .collect::<Vec<_>>();

    // println!("{:#?}", functions);

    Ok(())
}
