use std::io::{Read, Write};
use crate::daemon::DAEMON_FINGERPRINT_SIZE;
use crate::{DaemonFingerprint, DaemonResult as Result};
use assemble_core::fingerprint::Fingerprint;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::message::{RequestReceiver, ResponseSender};

/// Args for starting a daemon
#[derive(Debug, clap::Parser)]
pub struct DaemonArgs {
    /// The root build file to be used
    root_build_file: PathBuf,
}

/// The assemble daemon, controls actual execution. Usually ran in its own process;
pub struct Daemon<R : Read, W : Write> {
    receiver: RequestReceiver<R>,
    sender: ResponseSender<W>
}

impl<R : Read, W : Write> Daemon<R, W> {

    /// Start a new Daemon
    pub fn start(args: DaemonArgs) -> Result<Self> {
        todo!()
    }

    /// Find an existing daemon
    pub fn find(args: DaemonArgs) -> Result<Self> {
        todo!()
    }
    pub fn new(receiver: R, sender: W) -> Self {
        Self { receiver: RequestReceiver::new(receiver), sender: ResponseSender::new(sender) }
    }
}

impl<R : Read, W : Write> Fingerprint<DAEMON_FINGERPRINT_SIZE> for Daemon<R, W> {
    fn fingerprint(&self) -> DaemonFingerprint {
        todo!()
    }
}

/// Provides a connection to a daemon, and allows for send/receiving messages with Daemons
pub struct DaemonConnection<R : Read, W : Write> {
    daemon_input: W,
    daemon_output: R,
}

impl<R: Read, W: Write> DaemonConnection<R, W> {
    pub(crate) fn new(daemon_input: W, daemon_output: R) -> Self {
        Self { daemon_input, daemon_output }
    }
}
