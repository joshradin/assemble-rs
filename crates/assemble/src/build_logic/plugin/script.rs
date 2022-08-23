//! Create build scripts

use std::fs::File;
use std::io;
use std::io::Read;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

/// Marks a type as a scripting language
pub trait ScriptingLang: Default + Sized + 'static {
    /// Fina a build script in a path
    fn find_build_script(&self, in_dir: &Path) -> Option<PathBuf>;

    fn open_build_script(&self, path: &Path) -> Option<BuildScript<Self>> {
        if path.exists() && path.is_file() {
            Some(BuildScript::new(path))
        } else {
            None
        }
    }
}

/// Languages the implement ScriptingLang by default
pub mod languages {
    use std::path::{Path, PathBuf};

    use crate::build_logic::plugin::script::BuildScript;

    use super::ScriptingLang;

    /// Configure a project using `yaml`
    #[cfg(feature = "yaml")]
    #[derive(Debug, Default)]
    pub struct YamlLang;

    #[cfg(feature = "yaml")]
    impl ScriptingLang for YamlLang {
        fn find_build_script(&self, in_dir: &Path) -> Option<PathBuf> {
            let path = in_dir.join("assemble.build.yaml");
            if path.exists() && path.is_file() {
                Some(path)
            } else {
                None
            }
        }
    }

    pub struct RustLang;
}

/// A build script
pub struct BuildScript<L: ScriptingLang> {
    lang: PhantomData<L>,
    path: PathBuf,
    contents: Vec<u8>,
}

impl<L: ScriptingLang> Read for BuildScript<L> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let to_len = buf.len().min(self.contents.len());
        buf.clone_from_slice(&self.contents[..to_len]);
        Ok(to_len)
    }
}

impl<L: ScriptingLang> BuildScript<L> {
    /// Create a new build script at a path
    ///
    /// # Panic
    /// will panic if the file path can't be opened
    pub fn new<P: AsRef<Path>>(file_path: P) -> Self {
        let mut file = File::open(file_path.as_ref()).unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .expect("couldn't open and read file");
        Self {
            lang: PhantomData,
            path: file_path.as_ref().to_path_buf(),
            contents: buf,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn contents(&self) -> &[u8] {
        &self.contents
    }
}
