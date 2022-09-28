//! Provides the structures for creating connections

use std::convert::Infallible;
use std::error::Error;
use std::io;
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use crate::{connect, FromData, IntoData};
use crate::sockets::{LocalOrRemoteStream, Socketish};
use crate::streams::{RawStream, Stream};


/// A type that can produce sockets from incoming connections
pub trait ConnectionServer {
    type Socket : Socketish;
    type Err : Error;

    /// Creates a stream into an incoming stream.
    fn accept_incoming<R : FromData, W : IntoData>(&mut self) -> Result<RawStream<Self::Socket, R, W>, Self::Err>;
}

impl ConnectionServer for TcpListener {
    type Socket = TcpStream;
    type Err = io::Error;

    fn accept_incoming<R : FromData, W : IntoData>(&mut self) -> Result<RawStream<Self::Socket, R, W>, Self::Err> {
        let (stream, addr) = self.accept()?;
        Ok(RawStream::new(stream))
    }
}

impl ConnectionServer for LocalSocketListener {
    type Socket = LocalSocketStream;
    type Err = io::Error;

    fn accept_incoming<R: FromData, W: IntoData>(&mut self) -> Result<RawStream<Self::Socket, R, W>, Self::Err> {
        Ok(RawStream::new(self.accept()?))
    }
}

/// Turns something into a connection server
pub trait IntoConnectionServer {
    type ConnectionServer: ConnectionServer;
    type Err;

    /// Creating the server isn't guaranteed to work
    fn into_server(self) -> Result<Self::ConnectionServer, Self::Err>;
}

impl<I : ConnectionServer> IntoConnectionServer for I {
    type ConnectionServer = I;
    type Err = Infallible;

    fn into_server(self) -> Result<Self::ConnectionServer, Self::Err> {
        Ok(self)
    }
}

/// A listener that can either be local or remote
pub enum LocalOrRemoteListener {
    /// A local socket (os supported)
    Local(LocalSocketListener),
    /// A remote socket
    Remote(TcpListener)
}

impl ConnectionServer for LocalOrRemoteListener {
    type Socket = LocalOrRemoteStream;
    type Err = io::Error;

    fn accept_incoming<R: FromData, W: IntoData>(&mut self) -> Result<RawStream<Self::Socket, R, W>, Self::Err> {
        let socket = match self {
            LocalOrRemoteListener::Local(l) => {
                let socket = l.accept()?;
                LocalOrRemoteStream::Local(socket)
            }
            LocalOrRemoteListener::Remote(r) => {
                let (socket, _) = r.accept()?;
                LocalOrRemoteStream::Remote(socket)
            }
        };
        Ok(RawStream::bind(socket)?)
    }
}

/// An invalid socket address
#[derive(Debug, thiserror::Error)]
pub enum InvalidSocket {
    #[error("Invalid socket address: {0}")]
    InvalidAddress(String),
    #[error("Couldn't create server: {0}")]
    IoError(#[from] io::Error)
}

impl IntoConnectionServer for &str {
    type ConnectionServer = LocalOrRemoteListener;
    type Err = InvalidSocket;

    fn into_server(self) -> Result<Self::ConnectionServer, Self::Err> {
        if let Ok(addr) = SocketAddr::from_str(self) {
            let listener = TcpListener::bind(addr)?;
            Ok(LocalOrRemoteListener::Remote(listener))
        } else if let Ok(path) = Path::new(self).canonicalize() {
            let listener = LocalSocketListener::bind(path)?;
            Ok(LocalOrRemoteListener::Local(listener))
        } else {
            Err(InvalidSocket::InvalidAddress(self.to_string()))
        }
    }
}

impl IntoConnectionServer for SocketAddr {
    type ConnectionServer = TcpListener;
    type Err = io::Error;

    fn into_server(self) -> Result<Self::ConnectionServer, Self::Err> {
        TcpListener::bind(self)
    }
}

impl IntoConnectionServer for (&str, u16) {
    type ConnectionServer = TcpListener;
    type Err = io::Error;

    fn into_server(self) -> Result<Self::ConnectionServer, Self::Err> {
        TcpListener::bind(self)
    }
}

impl IntoConnectionServer for (String, u16) {
    type ConnectionServer = TcpListener;
    type Err = io::Error;

    fn into_server(self) -> Result<Self::ConnectionServer, Self::Err> {
        TcpListener::bind(self)
    }
}

impl IntoConnectionServer for (IpAddr, u16) {
    type ConnectionServer = TcpListener;
    type Err = io::Error;

    fn into_server(self) -> Result<Self::ConnectionServer, Self::Err> {
        TcpListener::bind(self)
    }
}

impl IntoConnectionServer for (Ipv4Addr, u16) {
    type ConnectionServer = TcpListener;
    type Err = io::Error;

    fn into_server(self) -> Result<Self::ConnectionServer, Self::Err> {
        TcpListener::bind(self)
    }
}

impl IntoConnectionServer for (Ipv6Addr, u16) {
    type ConnectionServer = TcpListener;
    type Err = io::Error;

    fn into_server(self) -> Result<Self::ConnectionServer, Self::Err> {
        TcpListener::bind(self)
    }
}

impl IntoConnectionServer for &Path {
    type ConnectionServer = LocalSocketListener;
    type Err = io::Error;

    fn into_server(self) -> Result<Self::ConnectionServer, Self::Err> {
        LocalSocketListener::bind(self)
    }
}