use assemble_core::defaults::sources::crate_registry::CrateName;
use assemble_core::dependencies::{Dependency, Source};
use assemble_core::project::Project;
use assemble_core::utilities::ArcExt;
use assemble_core::workspace::{Dir, Workspace, WorkspaceDirectory};
use flate2::read::GzDecoder;
use include_dir::{include_dir, Dir as IncludeDir};
use reqwest::blocking::Response;
use reqwest::header::CONTENT_DISPOSITION;
use serde::de::Error;
use serde::{Serialize, Serializer};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{Debug, Formatter};
use std::fs::ReadDir;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::slice::Iter;
use std::sync::{Arc, Mutex};
use tar::Archive;
use threadpool::ThreadPool;
use url::Url;

static RUST_RESOURCES: IncludeDir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/resources/rust");

/// Creates an assemble binary using rust/cargo
pub struct RustBinaryBuilder {
    working_directory: PathBuf,
    output_file: PathBuf,
    build_workspace: Workspace,
}

impl RustBinaryBuilder {
    pub fn create_rust_workspace(&self, deps: &Dependencies) {
        let cargo_file = self.build_workspace.file("Cargo.toml").unwrap();

        let dependencies_toml = toml::to_string(deps);
    }
}



#[derive(Serialize)]
pub struct Manifest {
    package: Package,
    dependencies: Dependencies,
}

#[derive(Serialize)]
pub struct Package {
    name: String,
    version: String,
}

#[derive(Default)]
pub struct Dependencies {
    dependencies: Vec<Box<dyn Dependency>>,
    dependency_to_path: HashMap<CrateName, PathBuf>,
    downloaded: bool,
}

impl Dependencies {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn add_dependency<D: 'static + Dependency>(&mut self, dependency: D) {
        self.dependencies.push(Box::new(dependency));
    }

    pub fn download(&mut self, num_cores: usize, download_workspace: &Workspace) -> bool {
        if self.downloaded {
            return true;
        }
        let dependencies = (&*self).into_iter().collect::<VecDeque<_>>();

        let thread_pool = ThreadPool::new(num_cores);

        let dependency_map = Arc::new(Mutex::new(HashMap::new()));

        for dependency in dependencies {
            let id = dependency.0;
            let url = dependency.1;

            let dependency_map = dependency_map.clone();
            let workspace = download_workspace.clone();

            thread_pool.execute(move || {
                println!("Downloading {} from {}", id, url);

                let download_path = workspace.as_dir();
                let mut response = reqwest::blocking::get(url).expect("couldnt download");
                let path = Self::handle_response(&id, response, download_path).unwrap();

                let mut guard = dependency_map.lock().unwrap();
                guard.insert(id, path);
            });
        }
        thread_pool.join();
        self.downloaded = thread_pool.panic_count() == 0;
        if self.downloaded {
            // let mut dependency_map_mutex = dependency_map.lock().unwrap();
            // let dependency_map = std::mem::replace(&mut *dependency_map_mutex, HashMap::new());
            self.dependency_to_path = dependency_map.consume();
        }
        thread_pool.panic_count() == 0
    }

    fn handle_response(
        package_name: &str,
        mut response: Response,
        downloads_path: Dir,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        use std::fs::File;
        use std::io;

        println!("Got response: {:#?}", response);

        let temp_dir = downloads_path
            .dir(".temp")
            .expect("couldn't create directory");

        let as_file = PathBuf::from(response.url().path());
        let file_name = as_file.file_name().unwrap();
        let file_name = file_name.to_str().unwrap();

        let mut crate_file = temp_dir.file(file_name).expect("Couldn't create file");
        io::copy(&mut response, &mut crate_file).expect("couldnt copy download");

        drop(crate_file);

        let crate_file = temp_dir.file(file_name).unwrap();

        match Path::new(file_name).extension() {
            Some(crate_ext) if crate_ext == "crate" || crate_ext == "tar" => {
                // println!("Extracting {crate_file:?} into {downloads_path:?}");
                let decompressed = GzDecoder::new(crate_file);
                let mut archive = Archive::new(decompressed);

                let unpack_path = downloads_path.dir(package_name).unwrap();

                archive
                    .unpack(&unpack_path.absolute_path())
                    .expect("Couldn't unpack crate tarball");

                // unpack path should have one child file

                let dir: ReadDir = std::fs::read_dir(&unpack_path.absolute_path())?;

                let child = dir.into_iter().next().unwrap()?;
                Ok(child.path())
            }
            Some(ext) => {
                panic!("Can't handle extension {:?}", ext);
            }
            None => {
                panic!("Don't know how to handle no extension");
            }
        }
    }
}

impl Debug for Dependencies {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut builder = f.debug_struct("Dependencies");
        builder.field("downloaded", &self.downloaded);
        if self.downloaded {
            let mut mapping = HashMap::new();
            for (crate_name, path) in &self.dependency_to_path {
                mapping.insert(crate_name, path);
            }
            builder.field("declared", &mapping);
        } else {
            let mut set = HashSet::new();
            for dependency in &self.dependencies {
                set.insert(dependency.id());
            }
            builder.field("declared", &set);
        }

        builder.finish()
    }
}

impl IntoIterator for &Dependencies {
    type Item = (String, Url);
    type IntoIter = <Vec<(String, Url)> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.dependencies
            .iter()
            .map(|dep| (dep.id().to_string(), dep.source()))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

impl Serialize for Dependencies {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let as_map: HashMap<_, _> = self
            .dependencies
            .iter()
            .map(|dependency| {
                let id = dependency.id();
                let mut map = HashMap::new();
                map.insert("path".to_string(), format!("dependencies/{}", id));
                (id, map)
            })
            .collect();
        as_map.serialize(serializer)
    }
}
