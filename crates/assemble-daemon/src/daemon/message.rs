//! The messages that are sent from and to Daemons

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

/// The request that is made to the daemon
#[derive(Debug, Deserialize, Serialize)]
pub enum Request {}

#[derive(Debug, Deserialize, Serialize)]
pub enum Response {}

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("Lost Connection")]
    Disconnection,
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

pub struct RequestReceiver<R: Read> {
    recv: R,
}

impl<R: Read> RequestReceiver<R> {
    pub fn new(recv: R) -> Self {
        Self { recv }
    }

    pub fn recv(&mut self) -> Result<Request, ConnectionError> {
        serde_json::from_reader::<_, Request>(&mut self.recv).map_err(ConnectionError::from)
    }
}

pub struct ResponseSender<W: Write> {
    send: W,
}

impl<W: Write> ResponseSender<W> {
    pub fn new(send: W) -> Self {
        Self { send }
    }

    pub fn send(&mut self, message: Response) -> Result<(), ConnectionError> {
        serde_json::to_writer(&mut self.send, &message).map_err(ConnectionError::from)
    }
}
