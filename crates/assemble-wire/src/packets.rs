//! Defines how packets are read and write

use std::collections::VecDeque;
use std::convert::Infallible;
use crate::error::Error;
use crate::{FromData, IntoData, RawStream, Socketish, Stream};
use std::io;
use crate::data::deserialize;
use crate::streams::read_until;

/// Headers are contained in every packet, and is always the same size
#[derive(Debug)]
pub struct Header {
    packet_length: u64,
    identifier: u64,
    is_reply: bool,
    code: Option<Error>,
}

impl Header {
    pub fn new(packet_length: u64, identifier: u64, is_reply: bool, code: Option<Error>) -> Self {
        Self { packet_length, identifier, is_reply, code }
    }
}

impl FromData for Header {
    type Err = Infallible;

    fn deserialize(bytes: &mut VecDeque<u8>) -> Result<Self, Self::Err> {
        let packet_length: u64 = deserialize(bytes)?;
        let identifier: u64 = deserialize(bytes)?;
        let is_reply: bool = deserialize(bytes)?;
        let code: Option<Error> = deserialize(bytes)?;

        Ok(Self {
            packet_length,
            identifier,
            is_reply,
            code,
        })
    }

    fn minimum_size() -> usize {
        u64::minimum_size() * 2 + bool::minimum_size() + <Option<Error>>::minimum_size()
    }

    fn maximum_size() -> Option<usize> {
        Some(u64::minimum_size() * 2 + bool::minimum_size() + <Option<Error>>::minimum_size())
    }
}

impl IntoData for Header {
    fn serialize(&self) -> Vec<u8> {
        let mut output = vec![];
        output.extend(self.packet_length.serialize());
        output.extend(self.identifier.serialize());
        output.extend(self.is_reply.serialize());
        output.extend(self.code.serialize());
        output
    }
}

impl Header {
    /// Try to read a header directly from a raw stream.
    pub fn read<S: Socketish, R: FromData, W: IntoData>(
        stream: &mut RawStream<S, R, W>,
    ) -> io::Result<Header> {
        let mut socket = stream.get_socket_mut();
        let bytes = VecDeque::from_iter(read_until(socket, Header::minimum_size())?);
        let header = Header::deserialize(&mut { bytes }).unwrap();
        Ok(header)
    }


    pub fn packet_length(&self) -> u64 {
        self.packet_length
    }

    pub fn data_length(&self) -> u64 {
        self.packet_length -
            Self::minimum_size() as u64
    }

    pub fn identifier(&self) -> u64 {
        self.identifier
    }
    pub fn is_reply(&self) -> bool {
        self.is_reply
    }
    pub fn code(&self) -> Option<&Error> {
        self.code.as_ref()
    }


}

/// A packet of data that sent between clients and servers
#[derive(Debug)]
pub struct Packet<T> {
    header: Header,
    data: T,
}

impl<T> Packet<T> {
    pub fn new(header: Header, data: T) -> Self {
        Self { header, data }
    }

    pub fn header(&self) -> &Header {
        &self.header
    }
    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn unwrap(self) -> (Header, T) {
        let Packet { header, data } = self;
        (header, data)
    }
}
impl Packet<Vec<u8>> {
    pub fn write<W : io::Write>(&self, mut writer: W) -> io::Result<()> {
        let header_bytes = self.header().serialize();
        writer.write_all(&header_bytes)?;
        writer.write_all(&self.data)?;
        writer.flush()
    }
}

