#![cfg(feature = "js")]

use assemble::builders::js::JavascriptBuilder;
use assemble::dev::FreightRunnerBuilder;
use assemble_core::Project;
use log::{info, LevelFilter};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn register_task() {
    env_logger::builder()
        .filter_level(LevelFilter::Debug)
        .init();
    let tempdir = tempfile::TempDir::new().unwrap();
    {
        let path = tempdir.path().to_path_buf();
        let buf = path.join("assemble.settings.js");
        info!("created settings file: {:?}", buf);
        let mut file = File::create(&buf).unwrap();
        {
            assert!(
                file.metadata().is_ok(),
                "settings file doesn't actually exist"
            );
            writeln!(file, "settings.root_project.name = 'test';").unwrap();
        }
        assert!(buf.exists(), "settings file should now exist");
        let mut builder = FreightRunnerBuilder::<JavascriptBuilder>::in_dir(&path).build();
        println!("builder: {builder:#?}");
        builder.default().expect("that could run default tasks");
    }
    drop(tempdir);
}
