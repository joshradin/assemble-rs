use crate::daemon::DAEMON_FINGERPRINT_SIZE;
use crate::{DaemonFingerprint, DaemonResult as Result};
use assemble_core::fingerprint::Fingerprint;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Args for starting a daemon
#[derive(Debug, clap::Parser)]
pub struct DaemonArgs {
    /// The root build file to be used
    root_build_file: PathBuf,
}

/// The assemble daemon, controls actual execution. Usually ran in its own process;
#[derive(Deserialize, Serialize)]
pub struct Daemon;

impl Daemon {
    /// Start a new Daemon
    pub fn start(args: DaemonArgs) -> Result<Self> {
        todo!()
    }

    /// Find an existing daemon
    pub fn find(args: DaemonArgs) -> Result<Self> {
        todo!()
    }
}

impl Fingerprint<DAEMON_FINGERPRINT_SIZE> for Daemon {
    fn fingerprint(&self) -> DaemonFingerprint {
        todo!()
    }
}
