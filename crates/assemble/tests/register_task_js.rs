#![cfg(feature = "js")]

use assemble::builders::js::JavascriptBuilder;
use assemble::dev::FreightRunnerBuilder;
use assemble_core::Project;

#[test]
fn register_task() {
    let mut builder = FreightRunnerBuilder::<JavascriptBuilder>::new().build();
    println!("builder: {builder:#?}");
    builder.default().expect("ok");
}
