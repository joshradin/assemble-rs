use assemble_core::logging::LOGGING_CONTROL;
use assemble_core::prelude::TaskId;
use assemble_std::specs::exec_spec::ExecSpecBuilder;

#[test]
fn emit_to_log() {
    let args = assemble_core::logging::LoggingArgs::default();
    args.init_root_logger().unwrap();
    if !log::log_enabled!(log::Level::Info) {
        panic!("log level info must be enabled")
    }
    LOGGING_CONTROL.with_origin(TaskId::new("task").unwrap(), || {
        let spec = ExecSpecBuilder::new()
            .with_exec("echo")
            .with_args(["hello", "world"])
            .build()
            .expect("Couldn't build exec spec");

        let result = { spec }.execute_spec("/").expect("Couldn't create handle");
        let wait = result.wait().expect("couldn't finish exec spec");
        assert!(wait.success());
    })
}
