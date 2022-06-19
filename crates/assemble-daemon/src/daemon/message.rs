//! The messages that are sent from and to Daemons

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::convert::Infallible;
use std::io;
use std::io::{Read, Write};

/// The request that is made to the daemon
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum Request {
    /// Just checks if a connection is valid. If no response, then not connected.
    IsConnected,

    /// Check if
    UpToDate
}

pub trait ReceiveRequest {
    type Error;

    fn recv(&mut self) -> Result<Request, Self::Error>;
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum Response {
    Boolean(bool),
}

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("Lost Connection")]
    Disconnection,
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

pub struct RequestReceiver<R: ReceiveRequest> {
    inner: R,
}

impl<R: ReceiveRequest> RequestReceiver<R> {
    pub fn new(recv: R) -> Self {
        Self { inner: recv }
    }

    pub fn recv(&mut self) -> Result<Request, R::Error> {
        self.inner.recv()
    }
}

impl RequestReceiver<RequestBuffer> {
    /// Create a request receiver for a assemble process created without use of a daemon server.
    pub fn serverless() -> Self {
        Self::new(RequestBuffer::with_capacity(16))
    }

    /// Queue a request into this receiver.
    pub fn queue_request(&mut self, request: Request) {
        self.inner.queue_request(request);
    }
}

impl<R> ReceiveRequest for R
where
    for<'a> &'a mut R: Read,
{
    type Error = ConnectionError;

    fn recv(&mut self) -> Result<Request, Self::Error> {
        serde_json::from_reader::<_, Request>(self).map_err(ConnectionError::from)
    }
}

pub trait SendResponse {
    type Error;

    fn send(&mut self, message: Response) -> Result<(), Self::Error>;
}

pub struct ResponseSender<W: SendResponse> {
    sender: W,
}

impl<W: SendResponse> ResponseSender<W> {
    pub fn new(send: W) -> Self {
        Self { sender: send }
    }

    pub fn send(&mut self, message: Response) -> Result<(), W::Error> {
        self.sender.send(message)
    }
}

impl ResponseSender<ResponseBuffer> {
    pub fn serverless() -> Self {
        Self::new(ResponseBuffer::new())
    }

    pub fn take_response(&mut self) -> Option<Response> {
        self.sender.get_response()
    }
}

impl<W> SendResponse for W
where
    for<'a> &'a mut W: Write,
{
    type Error = ConnectionError;

    fn send(&mut self, message: Response) -> Result<(), Self::Error> {
        serde_json::to_writer(self, &message).map_err(ConnectionError::from)
    }
}

/// A request receiver that's "attached" to `/dev/null`.
///
/// In reality, this is just a receiver that can send requests made within code, such that
/// serialization of requests isn't necessary. Useful for running daemons within the same
/// process.
///
/// Requests that are [queued] into this receiver are processed FIFO
///
/// [queued]: DevNullRequestReceiver::queue_request
pub struct RequestBuffer {
    buffer: VecDeque<Request>,
}

impl RequestBuffer {
    /// Creates a new request receiver with no heap allocation
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
        }
    }

    /// Creates anew request receiver with a preset capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
        }
    }

    /// Queues a request into this receiver.
    pub fn queue_request(&mut self, request: Request) {
        self.buffer.push_back(request);
    }
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
#[error(
    "Since connections to daemons are single threaded, the dev-null receiver wont work if empty."
)]
pub struct RequestBufferEmpty;

impl ReceiveRequest for RequestBuffer {
    type Error = RequestBufferEmpty;

    fn recv(&mut self) -> Result<Request, Self::Error> {
        if let Some(req) = self.buffer.pop_front() {
            Ok(req)
        } else {
            Err(RequestBufferEmpty)
        }
    }
}

/// This structure "sends" responses to itself, which can be later queried to get the response.
pub struct ResponseBuffer {
    buffer: VecDeque<Response>,
}

impl ResponseBuffer {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
        }
    }

    pub fn get_response(&mut self) -> Option<Response> {
        self.buffer.pop_front()
    }
}

impl SendResponse for ResponseBuffer {
    type Error = Infallible; // Should never fail

    fn send(&mut self, message: Response) -> Result<(), Self::Error> {
        self.buffer.push_back(message);
        Ok(())
    }
}

/// Used for nonfunctional receive and send requests
pub struct Empty;

impl ReceiveRequest for Empty {
    type Error = ();

    fn recv(&mut self) -> Result<Request, Self::Error> {
        Err(())
    }
}

impl SendResponse for Empty {
    type Error = ();

    fn send(&mut self, message: Response) -> Result<(), Self::Error> {
        Err(())
    }
}

#[cfg(test)]
mod test {
    use crate::message::{Request, RequestReceiver, SendResponse};

    #[test]
    fn serverless_receiver() {
        let mut receiver = RequestReceiver::serverless();
        receiver.queue_request(Request::IsConnected);
        receiver.queue_request(Request::IsConnected);

        assert_eq!(receiver.recv(), Ok(Request::IsConnected));
        assert_eq!(receiver.recv(), Ok(Request::IsConnected));
    }
}
