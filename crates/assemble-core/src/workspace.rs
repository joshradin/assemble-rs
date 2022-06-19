//! Workspaces help provide limited access to files

use crate::file::RegularFile;
use include_dir::DirEntry;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::env::temp_dir;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, PoisonError, RwLock};
use std::{io, path};
use tempfile::{Builder, TempDir};

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("Given path is protected, and can not be written to")]
    PathProtected(PathBuf),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error("Protected paths were poisoned")]
    PoisonError,
}

impl<T> From<PoisonError<T>> for WorkspaceError {
    fn from(_: PoisonError<T>) -> Self {
        Self::PoisonError
    }
}

pub type WorkspaceResult<T> = Result<T, WorkspaceError>;

pub trait WorkspaceEntry {
    fn into_absolute_path(self) -> WorkspaceResult<PathBuf>;
}

pub trait WorkspaceDirectory: WorkspaceEntry {
    /// Create a workspace that's relative to this workspace. Shares proteced paths with parent workspace
    /// and other created workspaces.
    ///
    /// # Error
    /// Will panic if any `..` paths are present.
    fn new_workspace<P: AsRef<Path>>(&self, path: P) -> Workspace {
        let resolved = self.my_workspace().resolve_path(path.as_ref());
        Workspace {
            root_dir: resolved,
            protected_path: self.my_workspace().protected_path.clone(),
        }
    }

    /// Gets the workspace this directory is part of.
    fn my_workspace(&self) -> &Workspace;

    /// Gets the path of this directory relative to the workspace.
    fn path(&self) -> PathBuf;

    /// Gets the absolute path of this directory
    fn absolute_path(&self) -> PathBuf {
        self.my_workspace().resolve_path(&self.path())
    }

    /// Creates a file within this directory
    ///
    /// # Error
    /// Will panic if `..` paths are present at root of workspace
    fn file(&self, file: &str) -> WorkspaceResult<RegularFile>;

    /// Creates a directory within this directory
    /// # Error
    /// Will panic if `..` paths are present at root of workspace
    fn dir(&self, name: &str) -> WorkspaceResult<Dir>;

    /// Creates a _protected_ directory in this directory
    /// # Error
    /// Will panic if `..` paths are present at root of workspace
    fn protected_dir(&self, name: &str) -> WorkspaceResult<Dir>;

    /// Creates a _protected_ file in this directory
    /// # Error
    /// Will panic if `..` paths are present at root of workspace
    fn protected_file(&self, name: &str) -> WorkspaceResult<RegularFile>;

    /// Checks if a path is protected.
    ///
    /// The path should be a relative path from the member.
    fn is_protected(&self, path: &Path) -> bool {
        self.my_workspace().is_protected(path)
    }
}

#[derive(Debug, Clone)]
pub struct Workspace {
    root_dir: PathBuf,
    protected_path: Arc<RwLock<HashSet<PathBuf>>>,
}

impl Workspace {
    /// Creates a workspace that's temporary
    pub fn new_temp() -> Self {
        let file = TempDir::new().unwrap();
        Self::new(file.into_path())
    }

    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            root_dir: path.as_ref().to_path_buf(),
            protected_path: Arc::new(Default::default()),
        }
    }

    /// Resolves a path relative to this workspace.
    ///
    /// '/' is treated as the workspace root.
    /// # Panic
    ///
    /// - Will panic if `..` present at root.
    /// - Will also panic if prefix is present (only on windows)
    pub fn resolve_path(&self, path: &Path) -> PathBuf {
        let origin = &self.root_dir;
        let mut relative = self.root_dir.clone();
        for component in path.components() {
            match component {
                Component::Prefix(_) => {
                    panic!("Prefix not supported")
                }
                Component::RootDir => {
                    relative = origin.clone();
                }
                Component::CurDir => {
                    // do nothing
                }
                Component::ParentDir => {
                    if &relative == origin {
                        panic!("Can't use .. from root of workspace")
                    }
                }
                Component::Normal(part) => relative.push(part),
            }
        }
        self.root_dir.join(relative)
    }

    pub fn is_protected(&self, path: &Path) -> bool {
        let guard = self
            .protected_path
            .read()
            .expect("Couldn't get protected paths");
        let resolved = self.resolve_path(path);
        guard.contains(&resolved)
    }

    fn protect_path(&self, file: &Path) -> Result<(), WorkspaceError> {
        if self.is_protected(file) {
            Err(WorkspaceError::PathProtected(file.to_path_buf()))
        } else {
            let mut guard = self.protected_path.write()?;
            let resolved = self.resolve_path(file);
            guard.insert(resolved);
            Ok(())
        }
    }

    fn create_file(&self, path: &Path) -> Result<RegularFile, WorkspaceError> {
        if self.is_protected(&path) {
            Err(WorkspaceError::PathProtected(path.to_path_buf()))
        } else {
            let path = self.resolve_path(path);
            let true_path = self.root_dir.join(path);
            RegularFile::with_options(
                true_path,
                OpenOptions::new().read(true).write(true).create(true),
            )
            .map_err(|e| e.into())
        }
    }

    pub fn as_dir(&self) -> Dir {
        self.dir("").unwrap()
    }

    pub fn join<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        self.root_dir.join(path)
    }
}

impl WorkspaceEntry for Workspace {
    fn into_absolute_path(self) -> WorkspaceResult<PathBuf> {
        std::fs::canonicalize(self.root_dir).map_err(|e| e.into())
    }
}

impl WorkspaceDirectory for Workspace {
    fn my_workspace(&self) -> &Workspace {
        self
    }

    fn path(&self) -> PathBuf {
        PathBuf::new()
    }

    fn file(&self, file: &str) -> WorkspaceResult<RegularFile> {
        let file_path = PathBuf::from(file);
        self.create_file(&file_path)
    }

    fn dir(&self, name: &str) -> WorkspaceResult<Dir> {
        let dir_path = PathBuf::from(name);
        if self.is_protected(&dir_path) {
            return Err(WorkspaceError::PathProtected(dir_path));
        }
        let resolved = self.resolve_path(&dir_path);
        std::fs::create_dir(resolved)?;
        Ok(Dir {
            workspace: self,
            dir_path,
        })
    }

    fn protected_dir(&self, name: &str) -> WorkspaceResult<Dir> {
        let output = self.dir(name)?;
        self.protect_path(&output.path())?;
        Ok(output)
    }

    fn protected_file(&self, name: &str) -> WorkspaceResult<RegularFile> {
        let output = self.file(name)?;
        let path = Path::new(name);
        self.protect_path(path)?;
        Ok(output)
    }
}

pub struct Dir<'w> {
    workspace: &'w Workspace,
    dir_path: PathBuf,
}

impl WorkspaceEntry for Dir<'_> {
    fn into_absolute_path(self) -> WorkspaceResult<PathBuf> {
        std::fs::canonicalize(self.workspace.resolve_path(&self.dir_path)).map_err(|e| e.into())
    }
}

impl<'w> WorkspaceDirectory for Dir<'w> {
    fn my_workspace(&self) -> &Workspace {
        self.workspace
    }

    fn path(&self) -> PathBuf {
        self.dir_path.clone()
    }

    fn file(&self, file: &str) -> WorkspaceResult<RegularFile> {
        let file_path = self.dir_path.join(file);
        self.workspace.create_file(&file_path)
    }

    fn dir(&self, name: &str) -> WorkspaceResult<Dir> {
        let dir_path = self.dir_path.join(name);
        std::fs::create_dir(self.workspace.resolve_path(&dir_path))?;
        if self.workspace.is_protected(&dir_path) {
            return Err(WorkspaceError::PathProtected(dir_path));
        }
        Ok(Dir {
            workspace: self.workspace,
            dir_path,
        })
    }

    fn protected_dir(&self, name: &str) -> WorkspaceResult<Dir> {
        let output = self.dir(name)?;
        self.workspace.protect_path(&output.path())?;
        Ok(output)
    }

    fn protected_file(&self, name: &str) -> WorkspaceResult<RegularFile> {
        let output = self.file(name)?;
        let path = Path::new(name);
        self.workspace.protect_path(path)?;
        Ok(output)
    }
}

/// The default workspaces provide access common workspaces used within assemble
pub mod default_workspaces {
    use crate::workspace::Workspace;
    use once_cell::sync::Lazy;
    use std::env;
    use std::ops::{Deref, DerefMut};
    use std::path::PathBuf;

    /// The environment variable checked for home directory of assemble.
    pub const ASSEMBLE_HOME_VAR: &str = "ASSEMBLE_HOME";
    const ASSEMBLE_HOME_DIR_NAME: &str = ".assemble";

    /// Provides access to the instance of the Assemble home workspace
    pub static ASSEMBLE_HOME: Lazy<AssembleHome> = Lazy::new(AssembleHome::default);

    /// Provide access to the Home workspace of the assemble application. This value
    /// is determined by the environment variable `ASSEMBLE_HOME`. If this variable is not set,
    /// `$HOME/.assemble` is used.
    #[derive(Debug)]
    pub struct AssembleHome(Workspace);

    impl AssembleHome {
        /// Gets an instance of [`AssembleHome`](Self).
        ///
        /// # Panic
        ///
        /// Will panic if `ASSEMBLE_HOME` and `HOME` isn't set.
        ///
        /// Will panic if the location doesn't exist and can't be created.
        ///
        /// Will panic if the location already exists but is a file.
        fn default() -> Self {
            let location = env::var_os(ASSEMBLE_HOME_VAR).map_or_else(
                || {
                    let home = dirs::home_dir()
                        .expect("HOME variable must be set is ASSEMBLE_HOME is not");
                    let path = PathBuf::from(home);
                    path.join(ASSEMBLE_HOME_DIR_NAME)
                },
                |assemble_home| PathBuf::from(assemble_home),
            );
            if !location.exists() {
                std::fs::create_dir_all(&location).unwrap();
            } else if location.is_file() {
                panic!(
                    "Can not use assemble home at {:?} because it already exists as a file",
                    location
                );
            }

            let workspace = Workspace::new(location);
            Self(workspace)
        }
    }

    impl Deref for AssembleHome {
        type Target = Workspace;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl DerefMut for AssembleHome {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
}

#[cfg(test)]
mod test {
    use crate::workspace::{Workspace, WorkspaceDirectory};

    #[test]
    fn create_file() {
        let mut workspace = Workspace::new_temp();
        let file = workspace.file("temp.text").unwrap();
        assert!(file.metadata().unwrap().is_file());
    }

    #[test]
    fn create_file_in_dir() {
        let mut workspace = Workspace::new_temp();
        let dir = workspace.dir("temp").unwrap();
        println!("absolute: {:?}", dir.absolute_path());
        assert!(dir.absolute_path().is_dir());
        let file = dir.file("tests.txt").unwrap();
        assert!(file.metadata().unwrap().is_file());
    }
}
