//! # Assemble Wire Protocols
//! Defines mechanisms, structs, and functions to connect different assemble applications together.
//! Ideally, this will allow for improving stuff like logging and file watching.

mod connections;
pub mod data;
pub mod error;
mod packets;
pub mod sockets;
mod streams;
#[cfg(feature = "handshake")]
pub mod handshake;


use std::io;
use std::marker::PhantomData;
use std::net::IpAddr;

pub use crate::connections::*;
use crate::sockets::Socketish;
use crate::streams::Stream;
pub use data::{FromData, IntoData};
use sockets::ToSocketish;
pub use streams::*;

/// Try to connect to a server
pub fn connect<R: FromData, W: IntoData, S: ToSocketish>(
    socket: S,
) -> io::Result<RawStream<S::Socket, R, W>> {
    todo!()
}

/// Creates a listener
pub fn listener<S: IntoConnectionServer>(server: S) -> Result<S::ConnectionServer, S::Err> {
    server.into_server()
}

pub struct Client<So: Socketish, R, W = R, Str = RawStream<So, R, W>>
where
    R: FromData,
    W: IntoData,
    Str: Stream<So, R, W>,
{
    _data: PhantomData<(R, W, Str)>,
    socket: So,
}

impl<So: Socketish, R, W, Str> Client<So, R, W, Str>
where
    R: FromData,
    W: IntoData,
    Str: Stream<So, R, W>,
{
    /// Create a new client
    pub fn new<S: ToSocketish<Socket = So>>(socket: S) -> Result<Self, S::Err> {
        Ok(Self {
            _data: PhantomData,
            socket: socket.into_socketish()?,
        })
    }

    /// Attempts to connect this client
    pub fn connect(self) -> Result<Str, Str::Err> {
        Str::bind(self.socket)
    }
}
mod logging {

    macro_rules! log {
        ($($tt:tt)*) => {
            #[cfg(feature = "trace")]
            ::log::log!($($tt)*);
        };
    }
    macro_rules! error {
        ($($tt:tt)*) => {
            log!(::log::Level::Error, $($tt)*)
        };
    }
    macro_rules! warning {
        ($($tt:tt)*) => {
            log!(::log::Level::Warn, $($tt)*)
        };
    }
    macro_rules! info {
        ($($tt:tt)*) => {
            log!(::log::Level::Info, $($tt)*)
        };
    }
    macro_rules! debug {
        ($($tt:tt)*) => {
            log!(::log::Level::Debug, $($tt)*)
        };
    }
    macro_rules! trace {
        ($($tt:tt)*) => {
            log!(::log::Level::Trace, $($tt)*)
        };
    }

    pub(crate) use {log, error, warning, trace, info ,debug};
}
pub(crate) use logging::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_local() {
        let mut listener = listener("127.0.0.1:8000").unwrap();
        let client = Client::<_, Vec<u8>>::new("127.0.0.1:8000")
            .unwrap()
            .connect()
            .unwrap();
        let stream = listener.accept_incoming::<Vec<u8>, Vec<u8>>().unwrap();
    }
}
