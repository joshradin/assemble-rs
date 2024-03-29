use crate::project::buildable::{Buildable, BuiltByContainer, IntoBuildable};
use std::fmt::{Debug, Display, Formatter};
use std::fs::{File, Metadata, OpenOptions};
use std::io;
use std::io::{Read, Write};

use std::path::{Path, PathBuf};

/// A wrapper type that derefs to a File, while also providing access to it's path
pub struct RegularFile {
    path: PathBuf,
    file: File,
    open_options: OpenOptions,
    built_by: BuiltByContainer,
}

assert_impl_all!(RegularFile: Send, Sync);

impl RegularFile {
    /// Create a regular file using an options object and a path
    pub fn with_options<P: AsRef<Path>>(path: P, options: &OpenOptions) -> io::Result<Self> {
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            file: options.open(path)?,
            open_options: options.clone(),
            built_by: BuiltByContainer::default(),
        })
    }

    /// Opens a file in write-only mode.
    ///
    /// Will create a file if it does not exist, and will truncate if it does.
    pub fn create<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        Self::with_options(
            path,
            File::options().create(true).write(true).truncate(true),
        )
    }

    /// Attempts to open a file in read-only mode.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        Self::with_options(path, File::options().read(true))
    }

    /// Gets the path of the file
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Add a built by
    pub fn built_by<T: Buildable + 'static>(&mut self, task: T) {
        self.built_by.add(task)
    }

    /// Get the underlying file of this regular file
    pub fn file(&self) -> &File {
        &self.file
    }

    pub fn metadata(&self) -> io::Result<Metadata> {
        self.file().metadata()
    }
}

impl Debug for RegularFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegularFile")
            .field("path", &self.path)
            .field("open_options", &self.open_options)
            .field("built_buy", &"...")
            .finish()
    }
}

impl From<RegularFile> for PathBuf {
    fn from(rf: RegularFile) -> Self {
        rf.path
    }
}

impl Display for RegularFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.path)
    }
}

impl Read for RegularFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}

impl Read for &RegularFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (&self.file).read(buf)
    }
}

impl Write for RegularFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl AsRef<Path> for RegularFile {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl IntoBuildable for &RegularFile {
    type Buildable = BuiltByContainer;

    fn into_buildable(self) -> Self::Buildable {
        self.built_by.clone()
    }
}

/// Trait to get this value as a file location
pub trait AsFileLocation {
    /// Some type that can be interpreted as a path
    type FilePath: AsRef<Path>;

    /// Get the file location of this value
    fn file_location(&self) -> Self::FilePath;
}

impl<P: AsRef<Path>> AsFileLocation for P {
    type FilePath = PathBuf;

    fn file_location(&self) -> Self::FilePath {
        self.as_ref().to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn create_file() {
        let tempdir = TempDir::new().unwrap();
        let file = RegularFile::with_options(
            tempdir.path().join("file"),
            OpenOptions::new().create(true).write(true),
        )
        .unwrap();

        assert_eq!(file.path(), tempdir.path().join("file"));
    }

    #[test]
    fn can_write() {
        let tempdir = TempDir::new().unwrap();
        let file = RegularFile::with_options(
            tempdir.path().join("file"),
            OpenOptions::new().create(true).write(true),
        )
        .unwrap();

        writeln!(file.file(), "Hello, World!").expect("Couldn't write to file");
    }

    #[test]
    fn can_read() {
        let tempdir = TempDir::new().unwrap();
        let reg_file = RegularFile::with_options(
            tempdir.path().join("file"),
            OpenOptions::new().create(true).write(true),
        )
        .unwrap();

        let mut file = reg_file.file();
        writeln!(file, "Hello, World!").expect("Couldn't write to file");

        let mut file =
            RegularFile::with_options(tempdir.path().join("file"), OpenOptions::new().read(true))
                .unwrap();

        let mut buffer = String::new();
        file.read_to_string(&mut buffer)
            .expect("Couldn't read from file");
        assert_eq!(buffer.trim(), "Hello, World!");
    }
}
