use assemble_core::exception::BuildException;
use assemble_core::logging::LOGGING_CONTROL;

use assemble_core::Project;
use assemble_freight::ops::execute_tasks;
use assemble_freight::FreightArgs;

use assemble_rust::plugin::RustBasePlugin;

use clap::Parser;
use log::info;

fn main() {
    let args: FreightArgs = FreightArgs::parse();
    let handle = args.logging().init_root_logger().unwrap();

    let project = Project::temp(None);
    project
        .apply_plugin::<RustBasePlugin>()
        .expect("couldn't apply rust plugin");

    let results = execute_tasks(&project, &args).unwrap();

    for result in results {
        match result.result.as_ref().map_err(|e| e.kind()) {
            Err(BuildException::Error(error)) => {
                info!("task {} failed", result.id);
                info!("reason: {}", error);
            }
            _ => {}
        }
    }

    if let Some(handle) = handle {
        LOGGING_CONTROL.stop_logging();
        handle.join().unwrap();
    }
}
