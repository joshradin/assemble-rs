use npm_rs::{NodeEnv, NpmEnv};
use std::path::Path;
use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=src/ts");
    println!("cargo:rerun-if-changed=build.rs");

    let out_dir = env::var("OUT_DIR").expect("should be set in build script");
    drop(fs::remove_dir_all(Path::new(&out_dir).join("js")));

    fs::copy("package.json", Path::new(&out_dir).join("package.json")).unwrap();

    let manifest = env::var("CARGO_MANIFEST_DIR").unwrap();
    let src = Path::new(&manifest).join("src/ts");

    let result = NpmEnv::default()
        .with_node_env(&NodeEnv::Production)
        .set_path(&out_dir)
        .init_env()
        .install(None)
        .run(&format!("build --  -p {src:?} --outDir {out_dir}/js"))
        .exec()
        .expect("could not run npm command. is npm installed?");
    assert!(result.success(), "could not build typescript project")
}
