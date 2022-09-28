//! Provides socketish

use std::convert::Infallible;
use std::error::Error;
use std::io::{Read, Write};
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::io;
use std::path::Path;
use std::str::FromStr;
use crate::{InvalidSocket, LocalOrRemoteListener};

/// Some value that could be interpreted as a socket-ish address
pub trait ToSocketish {
    type Socket: Socketish;
    type Err : Error;

    /// Turn this into a socket
    fn into_socketish(self) -> Result<Self::Socket, Self::Err>;
}

impl ToSocketish for &str {
    type Socket = LocalOrRemoteStream;
    type Err = InvalidSocket;

    fn into_socketish(self) -> Result<Self::Socket, Self::Err> {
        if let Ok(addr) = SocketAddr::from_str(self) {
            let listener = TcpStream::connect(addr)?;
            Ok(LocalOrRemoteStream::Remote(listener))
        } else if let Ok(path) = Path::new(self).canonicalize() {
            let listener = LocalSocketStream::connect(path)?;
            Ok(LocalOrRemoteStream::Local(listener))
        } else {
            Err(InvalidSocket::InvalidAddress(self.to_string()))
        }
    }
}

impl ToSocketish for ([u8; 4], u16) {
    type Socket = TcpStream;
    type Err = io::Error;

    fn into_socketish(self) -> Result<Self::Socket, Self::Err> {
        TcpStream::connect((Ipv4Addr::new(self.0[0], self.0[1], self.0[2], self.0[3]), self.1))
    }
}

impl<S : Socketish> ToSocketish for S {
    type Socket = Self;
    type Err = Infallible;

    fn into_socketish(self) -> Result<Self::Socket, Self::Err> {
        Ok(self)
    }
}

impl ToSocketish for SocketAddr {
    type Socket = TcpStream;
    type Err = io::Error;

    fn into_socketish(self) -> Result<Self::Socket, Self::Err> {
        TcpStream::connect(self)
    }
}

impl ToSocketish for (&str, u16) {
    type Socket = TcpStream;
    type Err = io::Error;

    fn into_socketish(self) -> Result<Self::Socket, Self::Err> {
        TcpStream::connect(self)
    }
}

/// Something that is "socketish" is something that can be both read and write to.
pub trait Socketish: Read + Write {}

// impl<T: Read + Write> Socketish for T {}

impl Socketish for TcpStream {

}

impl Socketish for LocalSocketStream {

}

impl Socketish for LocalOrRemoteStream { }

/// A socket that can either be local or remote
pub enum LocalOrRemoteStream {
    /// A local socket (os supported)
    Local(LocalSocketStream),
    /// A remote socket
    Remote(TcpStream)
}

impl Read for LocalOrRemoteStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        todo!()
    }
}

impl Write for LocalOrRemoteStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        todo!()
    }

    fn flush(&mut self) -> io::Result<()> {
        todo!()
    }
}

