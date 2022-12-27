use assemble_core::logging::opts::LoggingOpts;
use assemble_core::logging::LOGGING_CONTROL;
use assemble_core::prelude::TaskId;
use assemble_core::task::ExecutableTask;
use assemble_core::Project;
use assemble_rust::plugin::RustBasePlugin;

#[test]
#[ignore]
fn download_and_run_rustup() {
    let project = Project::temp(None);
    let handle = LoggingOpts::default().init_root_logger().unwrap();

    project.apply_plugin::<RustBasePlugin>().unwrap();

    let mut install_rustup = project
        .task_container()
        .get_task(&TaskId::new("install-rustup").unwrap())
        .cloned()
        .unwrap()
        .resolve_shared(&project)
        .unwrap();

    let result = project.with(|p| install_rustup.execute(p));
    assert!(result.is_ok(), "{}", result.unwrap_err());

    if let Some(handle) = handle {
        LOGGING_CONTROL.stop_logging();
        handle.join().unwrap();
    }
}
