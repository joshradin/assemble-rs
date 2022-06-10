use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use lockfile::{Lockfile};
use once_cell::sync::Lazy;
use assemble_core::{ASSEMBLE_HOME, Workspace};
use assemble_core::workspace::default_workspaces::AssembleHome;
use assemble_core::workspace::WorkspaceDirectory;
use crate::{Daemon, DaemonFingerprint, DaemonResult, RecoverState};
use serde::{Deserialize, Serialize};
use assemble_core::fingerprint::Fingerprint;

const DAEMON_SERVER_DIRECTORY: &str = "daemons";

#[derive(Debug)]
pub struct DaemonServer {
    workspace: Workspace,
    lock_file: Lockfile,

    index: DaemonServerIndex
}

impl DaemonServer {

    fn new(workspace: Workspace) -> Result<Self, DaemonServerError> {
        let lock_file = Lockfile::create(workspace.join(".lock"))?;
        let index = DaemonServerIndex::get(&workspace.absolute_path())?;
        Ok(Self {
            workspace,
            lock_file,
            index
        })
    }

    pub fn instance() -> &'static Self {
        static SERVER_INSTANCE: Lazy<DaemonServer> = Lazy::new(|| {
            let servers_workspace = ASSEMBLE_HOME.new_workspace(DAEMON_SERVER_DIRECTORY);
            let servers_path = servers_workspace.path();

            match DaemonServer::try_recover(&servers_path) {
                Ok(server) => {
                    server
                }
                Err(_) => {
                    if servers_path.exists() {
                        std::fs::remove_dir_all(servers_path).unwrap();
                    }
                    DaemonServer::new(servers_workspace).unwrap()
                }
            }
        });

        SERVER_INSTANCE.deref()
    }

}

impl RecoverState for DaemonServer {
    type Err = DaemonServerError;

    fn try_recover(path: &Path) -> Result<Self, Self::Err> {
        let index = DaemonServerIndex::get(path)?;
        for path in index.daemon_paths() {
            if !path.exists() {
                return Err(DaemonServerError::EntryMissing);
            }
        }
        let lockfile = Lockfile::create(path.join(".lock"))?;
        println!("recovered");
        Ok(Self {
            workspace: Workspace::new(path),
            lock_file: lockfile,
            index
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DaemonServerIndex {
    index_file: PathBuf,
    map: HashMap<DaemonFingerprint, PathBuf>
}


impl DaemonServerIndex {

    fn get(workspace: &Path) -> Result<Self, DaemonServerError> {
        let index_file = workspace.join(".index");
        let map = if index_file.exists() {
            serde_json::from_reader(File::open(&index_file)?)?
        } else {
            HashMap::new()
        };
        Ok(Self {
            index_file,
            map
        })
    }

    fn add_daemon<R : Read, W : Write>(&mut self, daemon: &Daemon<R, W>) -> Result<(), io::Error> {
        let finger_print = daemon.fingerprint();

        Ok(())
    }

    fn update_index_file(&mut self) {

    }

    fn daemon_paths(&self) -> impl Iterator<Item=&Path> {
        self.map.values().map(|path| path.as_path())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DaemonServerError {
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error("Entry within Daemon Server Index is Missing")]
    EntryMissing,
}

impl From<lockfile::Error> for DaemonServerError {
    fn from(e: lockfile::Error) -> Self {
        e.into_inner().into()
    }
}

#[cfg(test)]
mod tests {
    use assemble_core::Workspace;
    use assemble_core::workspace::WorkspaceDirectory;
    use crate::daemon_server::DaemonServer;
    use crate::RecoverState;

    #[test]
    fn can_get_instance() {
        let instance = DaemonServer::instance();
    }

    #[test]
    fn can_recover_instance() {
        let workspace = Workspace::new_temp();
        let daemon_server = DaemonServer::new(workspace.clone()).unwrap();

        println!("server: {:#?}", daemon_server);

        let saved_path = daemon_server.index.index_file.clone();
        drop(daemon_server);

        let recovered = DaemonServer::try_recover(&workspace.absolute_path()).expect("couldn't recover");
        assert_eq!(recovered.index.index_file, saved_path);
    }
}