use assemble_core::defaults::tasks::Empty;
use assemble_core::logging::{LoggingArgs, LOGGING_CONTROL};
use assemble_core::task::task_container::FindTask;
use assemble_core::task::{ExecutableTask, ResolveInnerTask};
use assemble_core::Project;
use assemble_rust::plugin::RustPlugin;

#[test]
fn download_and_run_rustup() {
    let project = Project::temp(None);
    let mut handle = LoggingArgs::default().init_root_logger().unwrap();

    project.apply_plugin::<RustPlugin>().unwrap();

    let mut install_rustup = project
        .task_container()
        .get_task("install-rustup")
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
