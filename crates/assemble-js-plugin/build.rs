use npm_rs::{NodeEnv, NpmEnv};
use std::path::Path;
use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=src/ts");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = env::var("OUT_DIR").expect("should be set in build script");
    let js_out_dir = Path::new(&out_dir).join("js");

    if js_out_dir.exists() {
        fs::remove_dir_all(&js_out_dir).expect("couldn't clean");
    }



    let result = NpmEnv::default()
        .with_node_env(&NodeEnv::Production)
        .init_env()
        .install(None)
        .run(&format!("build -- {}", js_out_dir.to_str().unwrap()))
        .exec()
        .expect("could not run npm command. is npm installed?");
    assert!(result.success(), "could not build typescript project")
}
