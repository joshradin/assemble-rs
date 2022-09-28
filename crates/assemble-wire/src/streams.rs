//! The protocol creates a reader or writer to create connections.

use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::fmt::Display;
use std::hash::Hash;
use crate::packets::{Header, Packet};
use crate::sockets::Socketish;
use crate::{FromData, IntoData, ToSocketish};
use std::io;
use std::io::{ErrorKind, Read, Write};
use std::marker::PhantomData;


/// Defines a stream
pub trait Stream<S: Socketish, R: FromData, W: IntoData>: Sized {
    type Err : Error;

    /// Bind a stream to a socket, performing a handshake
    fn bind<Sc : ToSocketish<Socket=S>>(socket: Sc) -> Result<Self, Self::Err>;

    /// Read a packet from the stream
    fn read(&mut self) -> Result<Packet<R>, Self::Err>;

    /// Send a packet to the stream
    fn send(&mut self, data: W) -> io::Result<()>;
}

/// A stream that allows for reading and writing. Has no protection.
pub struct RawStream<S: Socketish, R: FromData, W: IntoData = R> {
    socket: S,
    _data_types: PhantomData<(R, W)>,
}

impl<S: Socketish, R: FromData, W: IntoData> RawStream<S, R, W> {
    pub(crate) fn new(socket: S) -> Self {
        Self {
            socket,
            _data_types: Default::default(),
        }
    }

    pub fn from<R2 : FromData, W2 : IntoData>(raw: RawStream<S, R2, W2>) -> Self {
        RawStream {
            socket: raw.socket,
            _data_types: Default::default()
        }
    }

    /// Gets the underlying socket
    pub(crate) fn get_socket(&self) -> &S {
        &self.socket
    }

    /// Gets a mutable reference to the underlying socket
    pub(crate) fn get_socket_mut(&mut self) -> &mut S {
        &mut self.socket
    }

}

impl<S: Socketish, R: FromData, W: IntoData> Stream<S, R, W> for RawStream<S, R, W> {
    type Err = io::Error;

    fn bind<Sc: ToSocketish<Socket=S>>(socket: Sc) -> Result<Self, Self::Err> {
        let socket = socket.into_socketish().map_err(|e| io::Error::new(ErrorKind::ConnectionRefused, e.to_string()))?;
        Ok(Self::new(socket))
    }


    fn read(&mut self) -> Result<Packet<R>, Self::Err> {
        let header = Header::read(self)?;
        let data_size = header.data_length() as usize;
        let mut data = VecDeque::from_iter(read_until(self.get_socket_mut(), data_size)?);

        println!("received {:x?}", data);

        let deserialized = R::deserialize(&mut data)
            .map_err(|e| io::Error::new(ErrorKind::InvalidData, "couldn't deserialize data"))?;
        Ok(Packet::new(header, deserialized))
    }

    fn send(&mut self, data: W) -> io::Result<()> {
        let data = data.serialize();
        println!("sending {:x?}", data);
        let data_size = Header::minimum_size() + data.len();
        let header = Header::new(data_size as u64, 0, false, None);
        let packet = Packet::new(header, data);
        packet.write(&mut self.socket)
    }
}

/// Read until a certain number of bytes have been gotten. Errors if not the exact amount is gotten
pub fn read_until<R: Read>(mut reader: R, count: usize) -> io::Result<Vec<u8>> {
    let mut output = vec![];
    for _ in 0..count {
        let byte = (&mut reader)
            .bytes()
            .next()
            .ok_or(io::Error::new(
                ErrorKind::Interrupted,
                "Connection to host ended",
            ))
            .and_then(|s| s)?;

        output.push(byte);
    }
    Ok(output)
}


