use assemble_api::dependencies::{Dependency, Source};
use assemble_api::project::Project;
use assemble_api::workflow::BinaryBuilder;
use flate2::read::GzDecoder;
use include_dir::{include_dir, Dir};
use reqwest::blocking::Response;
use reqwest::header::CONTENT_DISPOSITION;
use serde::de::Error;
use serde::{Serialize, Serializer};
use std::collections::{HashMap, VecDeque};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::slice::Iter;
use tar::Archive;
use threadpool::ThreadPool;
use url::Url;

static RUST_RESOURCES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/resources/rust");

/// Creates an assemble binary using rust/cargo
pub struct RustBinaryBuilder {
    working_directory: PathBuf,
    output_file: PathBuf,
}

impl RustBinaryBuilder {
    pub fn create_rust_workspace(&self, deps: &Dependencies) {}
}

impl BinaryBuilder for RustBinaryBuilder {
    type Error = ();

    fn build_binary(self, project: Project) -> Result<(), Self::Error> {
        todo!()
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

pub struct Dependencies {
    dependencies: Vec<Box<dyn Dependency>>,
    downloaded: bool,
}

impl Dependencies {
    pub fn new() -> Self {
        Self {
            dependencies: vec![],
            downloaded: false,
        }
    }

    pub fn add_dependency<D: 'static + Dependency>(&mut self, dependency: D) {
        self.dependencies.push(Box::new(dependency));
    }

    pub fn download(&mut self, num_cores: usize, download_path: &Path) -> bool {
        if self.downloaded {
            return true;
        }
        let dependencies = (&*self).into_iter().collect::<VecDeque<_>>();

        let thread_pool = ThreadPool::new(num_cores);

        for dependency in dependencies {
            let id = dependency.0;
            let url = dependency.1;

            let download_path = download_path.to_path_buf();

            thread_pool.execute(move || {
                println!("Downloading {} from {}", id, url);
                let mut response = reqwest::blocking::get(url).expect("couldnt download");
                Self::handle_response(&id, response, &download_path).unwrap();
            });
        }
        thread_pool.join();
        self.downloaded = thread_pool.panic_count() == 0;
        thread_pool.panic_count() == 0
    }

    fn handle_response(
        package_name: &str,
        mut response: Response,
        downloads_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs::File;
        use std::io;

        println!("Got response: {:#?}", response);

        let temp_dir = downloads_path.join(".temp");

        std::fs::create_dir(&temp_dir);

        let crate_download_path = {
            let as_file = PathBuf::from(response.url().path());
            let file_name = as_file.file_name().unwrap();
            temp_dir.join(file_name)
        };

        println!("Download path = {:?}", crate_download_path);

        let mut crate_file = File::create(&crate_download_path).expect("Couldn't create file");
        io::copy(&mut response, &mut crate_file).expect("couldnt copy download");

        drop(crate_file);

        let crate_file = File::open(&crate_download_path).unwrap();

        match crate_download_path.extension() {
            Some(crate_ext) if crate_ext == "crate" || crate_ext == "tar" => {
                println!("Extracting {crate_file:?} into {downloads_path:?}");
                let decompressed = GzDecoder::new(crate_file);
                let mut archive = Archive::new(decompressed);
                archive
                    .unpack(downloads_path)
                    .expect("Couldn't unpack crate tarball");
            }
            Some(ext) => {
                panic!("Can't handle extension {:?}", ext);
            }
            None => {
                panic!("Don't know how to handle no extension");
            }
        };

        Ok(())
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
