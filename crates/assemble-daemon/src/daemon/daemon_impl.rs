use crate::daemon::DAEMON_FINGERPRINT_SIZE;
use crate::message::{
    ConnectionError, Empty, ReceiveRequest, Request, RequestBuffer, RequestReceiver, Response,
    ResponseBuffer, ResponseSender, SendResponse,
};
use crate::{DaemonError, DaemonFingerprint, DaemonResult as Result};
use assemble_core::fingerprint::Fingerprint;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io::{Read, Write};
use std::path::PathBuf;

/// Args for starting a daemon
#[derive(Debug, clap::Parser)]
pub struct DaemonArgs {
    /// The root build file to be used
    root_build_file: PathBuf,
}

/// The assemble daemon, controls actual execution. Usually ran in its own process;
pub struct Daemon<R: ReceiveRequest = Empty, W: SendResponse = Empty> {
    receiver: RequestReceiver<R>,
    sender: ResponseSender<W>,
}

impl<R: ReceiveRequest, W: SendResponse> Daemon<R, W> {
    pub fn new(receiver: R, sender: W) -> Self {
        Self {
            receiver: RequestReceiver::new(receiver),
            sender: ResponseSender::new(sender),
        }
    }

    pub fn process_request(&mut self) -> Result<()>
    where
        DaemonError: From<<W as SendResponse>::Error> + From<<R as ReceiveRequest>::Error>,
    {
        match self.receiver.recv() {
            Ok(req) => {
                let response = self.respond_to_request(req)?;
                self.sender.send(response)?;
                Ok(())
            }
            Err(e) => return Err(e.into()),
        }
    }

    fn respond_to_request(&mut self, request: Request) -> Result<Response> {
        match request {
            Request::IsConnected => Ok(Response::Boolean(true)),
        }
    }

    pub fn receiver(&mut self) -> &mut RequestReceiver<R> {
        &mut self.receiver
    }
    pub fn sender(&mut self) -> &mut ResponseSender<W> {
        &mut self.sender
    }
}

impl Daemon<RequestBuffer, ResponseBuffer> {
    /// Creates a Daemon without a server.
    ///
    /// Still requires interacting with the receiver and sender, but allows for buffering of responses.
    pub fn serverless() -> Self {
        Self::new(RequestBuffer::new(), ResponseBuffer::new())
    }

    pub fn queue(&mut self, requst: Request) {
        self.receiver.queue_request(requst);
    }
}

impl Daemon<Empty, Empty> {
    /// Creates a "local" daemon.
    ///
    /// Can not receive requests or send responses, and instead can only use the main driver function.
    pub fn local() -> Self {
        Self::new(Empty, Empty)
    }

    /// Process a request and get the response
    pub fn execute(&mut self, request: Request) -> Result<Response> {
        self.respond_to_request(request)
    }
}

impl<R: ReceiveRequest, W: SendResponse> Fingerprint<DAEMON_FINGERPRINT_SIZE> for Daemon<R, W> {
    fn fingerprint(&self) -> DaemonFingerprint {
        todo!()
    }
}

/// Provides a connection to a daemon, and allows for send/receiving messages with Daemons
pub struct DaemonConnection<R: Read, W: Write> {
    daemon_input: W,
    daemon_output: R,
}

impl<R: Read, W: Write> DaemonConnection<R, W> {
    pub(crate) fn new(daemon_input: W, daemon_output: R) -> Self {
        Self {
            daemon_input,
            daemon_output,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::message::{Request, Response};
    use crate::Daemon;

    #[test]
    fn serverless_can_buffer() {
        let mut daemon = Daemon::serverless();
        daemon.queue(Request::IsConnected);
        daemon
            .process_request()
            .expect("Should be able to process simple request");
        let response = daemon
            .sender
            .take_response()
            .expect("should have a response");
        assert_eq!(response, Response::Boolean(true));
    }

    #[test]
    fn local_daemon() {
        let mut daemon = Daemon::local();
        let response = daemon
            .respond_to_request(Request::IsConnected)
            .expect("should get response");
        assert_eq!(response, Response::Boolean(true));
    }

    #[test]
    #[should_panic]
    fn receiver_nonfunctional_in_local() {
        let mut daemon = Daemon::local();
        let _ = daemon.receiver.recv().unwrap();
    }

    #[test]
    #[should_panic]
    fn sender_nonfunctional_in_local() {
        let mut daemon = Daemon::local();
        daemon.sender.send(Response::Boolean(false)).unwrap();
    }
}
