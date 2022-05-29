use assemble_api::defaults::sources::crate_registry::{CrateRegistry, CrateUnresolvedDependency};
use assemble_api::dependencies::{DependencyResolver, DependencyResolverFactory};
use assemble_bin_maker::internal::cargo_backend::Dependencies;
use assemble_bin_maker::internal::dependencies::DefaultDependencyResolverFactory;
use std::path::Path;
use tempdir::TempDir;

#[test]
fn download_dependencies() {
    let directory = Path::new("cargo_dependencies");
    std::fs::remove_dir_all(directory);
    std::fs::create_dir(directory);

    ::new("cargo_dependencies").unwrap();

    let registry = CrateRegistry::crates_io();
    println!("using registry: {:?}", registry);

    let mut factory = DefaultDependencyResolverFactory::new();
    factory.add_source(registry);

    let resolver = factory.get_resolver();

    let dependency = CrateUnresolvedDependency::new("rand".to_string(), "0.8.2".to_string());

    let resolved = resolver
        .resolve_dependency(dependency)
        .expect("couldnt resolve dependency rand");

    println!("Resolved to {:#?}", resolved);

    let mut dependencies = Dependencies::new();
    dependencies.add_dependency(resolved);

    if !dependencies.download(1, directory.path()) {
        panic!("Could not download rand")
    }
}
