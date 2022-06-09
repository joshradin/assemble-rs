use std::fs::{File, OpenOptions};
use std::io;
use std::io::{Read, Write};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::fmt::{Display, Formatter};

/// A wrapper type that derefs to a File, while also providing access to it's path
#[derive(Debug)]
pub struct RegularFile {
    path: PathBuf,
    file: File,
}

impl RegularFile {
    /// Create a regular file using an options object and a path
    pub fn with_options<P: AsRef<Path>>(path: P, options: &OpenOptions) -> io::Result<Self> {
        let file = options.open(&path)?;
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            file,
        })
    }

    /// Opens a file in write-only mode.
    ///
    /// Will create a file if it does not exist, and will truncate if it does.
    pub fn create<P: AsRef<Path>>(path: P) -> io::Result<Self>  {
        Self::with_options(
            path,
        File::options().create(true).truncate(true)
        )
    }

    /// Attempts to open a file in read-only mode.
    pub fn open<P : AsRef<Path>>(path: P) -> io::Result<Self> {
        Self::with_options(
            path,
            File::options()
                .read(true)
        )
    }

    /// Gets the path of the file
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Deref for RegularFile {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

impl DerefMut for RegularFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.file
    }
}

impl AsRef<Path> for RegularFile {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl AsRef<File> for RegularFile {
    fn as_ref(&self) -> &File {
        &self.file
    }
}

impl Display for RegularFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.path)
    }
}

impl Read for RegularFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        (&self.file).read(buf)
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

impl Write for &RegularFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (&self.file).write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        (&self.file).flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;
    use std::io::Write;
    use std::io::Read;

    #[test]
    fn create_file() {
        let tempdir = TempDir::new("test").unwrap();
        let file = RegularFile::with_options(
            tempdir.path().join("file"),
            OpenOptions::new().create(true).write(true),
        )
        .unwrap();

        assert_eq!(file.path(), tempdir.path().join("file"));
    }

    #[test]
    fn can_write() {
        let tempdir = TempDir::new("test").unwrap();
        let mut file = RegularFile::with_options(
            tempdir.path().join("file"),
            OpenOptions::new().create(true).write(true),
        )
            .unwrap();

        writeln!(file, "Hello, World!").expect("Couldn't write to file");
    }

    #[test]
    fn can_read() {
        let tempdir = TempDir::new("test").unwrap();
        let mut file = RegularFile::with_options(
            tempdir.path().join("file"),
            OpenOptions::new().create(true).write(true),
        )
            .unwrap();

        writeln!(file, "Hello, World!").expect("Couldn't write to file");

        let mut file = RegularFile::with_options(
            tempdir.path().join("file"),
            OpenOptions::new().read(true),
        )
            .unwrap();

        let mut buffer = String::new();
        file.read_to_string(&mut buffer).expect("Couldn't read from file");
        assert_eq!(buffer.trim(), "Hello, World!");
    }
}
