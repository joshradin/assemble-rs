use assemble_bin_maker::internal::cargo_backend::Dependencies;
use assemble_bin_maker::internal::dependencies::DefaultDependencyResolverFactory;
use assemble_core::defaults::sources::crate_registry::{CrateRegistry, CrateUnresolvedDependency};
use assemble_core::dependencies::{DependencyResolver, DependencyResolverFactory};
use assemble_core::workspace::Workspace;
use std::path::Path;
use tempdir::TempDir;

#[test]
fn download_dependencies() {
    let directory = TempDir::new("cargo_dependencies").unwrap();

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

    let workspace = Workspace::new(directory);

    if !dependencies.download(1, &workspace) {
        panic!("Could not download rand")
    }

    println!("Dependencies = {:#?}", dependencies);
}
