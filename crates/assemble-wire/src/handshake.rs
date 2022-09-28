//! Provides a stream and a listener that requires a handshake first

use std::collections::VecDeque;
use crate::packets::Packet;
use crate::{
    ConnectionServer, FromData, IntoConnectionServer, IntoData, RawStream, Socketish, Stream,
    ToSocketish,
};
use std::error::Error;
use std::io;
use std::io::ErrorKind;
use std::marker::PhantomData;

use aes_gcm::{aead::{Aead, KeyInit, OsRng, AeadCore}, Aes256Gcm, Key, Nonce};
use typenum::U12;

/// A stream encrypted using Aes256Gcm. Establishes a key during the binding phase.
pub struct HandshakeStream<S: Socketish, R: FromData, W: IntoData = R> {
    raw_stream: RawStream<S, (Nonce<U12>, Vec<u8>)>,
    cipher: Aes256Gcm,
    data: PhantomData<(R, W)>,
}

impl<S: Socketish, R: FromData, W: IntoData> HandshakeStream<S, R, W> {

    /// Unwraps the stream, getting the underlying raw stream.
    pub(crate) fn unwrap(self) -> RawStream<S, R, W> {
        RawStream::from(self.raw_stream)
    }
}

impl<S: Socketish, R: FromData, W: IntoData> Stream<S, R, W> for HandshakeStream<S, R, W> {
    type Err = HandshakeError<<RawStream<S, Vec<u8>, Vec<u8>> as Stream<S, Vec<u8>, Vec<u8>>>::Err>;

    fn bind<Sc : ToSocketish<Socket=S>>(socket: Sc) -> Result<Self, Self::Err> {
        let mut raw = RawStream::<_, Vec<u8>>::bind(socket)?;
        let early: Packet<Vec<u8>> = raw.read()?;
        let key: Key<Aes256Gcm> = early.data().into_iter().map(|&b| b).collect();
        let cipher = Aes256Gcm::new(&key);
        raw.send(early.data().clone())?;
        Ok(Self {
            raw_stream: RawStream::from(raw),
            cipher,
            data: PhantomData
        })
    }

    fn read(&mut self) -> Result<Packet<R>, Self::Err> {
        let raw_packet = self.raw_stream.read()?;
        let nonce = &raw_packet.data().0;
        let data = raw_packet.data().1.clone();
        let mut decrypted = self.cipher.decrypt(nonce, &data[..])?;
        let data = R::deserialize(&mut VecDeque::from_iter(decrypted)).map_err(|e|
            HandshakeError::HandshakeFailed
        )?;
        let (header, _) = raw_packet.unwrap();
        Ok(Packet::new(header, data))
    }

    fn send(&mut self, data: W) -> io::Result<()> {
        let serialized = data.serialize();
        println!("original = {:x?}", serialized);
        let ref nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let encrypted = self.cipher.encrypt(nonce, &serialized[..])
            .map_err(|e| io::Error::new(ErrorKind::InvalidData, "couldn't deserialize data"))?;
        println!("encrypted = {:x?}", encrypted);
        self.raw_stream.send((nonce.clone(), encrypted))
    }
}

pub struct HandshakeListener<C: ConnectionServer> {
    server: C,
}

impl<C: ConnectionServer> HandshakeListener<C> {
    pub fn new<T: IntoConnectionServer<ConnectionServer = C>>(server: T) -> Result<Self, T::Err> {
        Ok(Self {
            server: server.into_server()?,
        })
    }
    pub fn accept<R: FromData, W: IntoData>(
        &mut self,
    ) -> Result<HandshakeStream<C::Socket, R, W>, HandshakeError<C::Err>> {
        let mut raw = self
            .server
            .accept_incoming::<Vec<u8>, Vec<u8>>()
            .map_err(HandshakeError::new)?;

        let key = Aes256Gcm::generate_key(&mut OsRng);
        let key_buffer: Vec<u8> = Vec::from_iter(key);

        raw.send(key_buffer.clone())?;
        let back = raw.read()?;

        if back.data() != &key_buffer {
            return Err(HandshakeError::HandshakeFailed);
        }

        let cipher = Aes256Gcm::new(&key);
        Ok(HandshakeStream {
            raw_stream: RawStream::from(raw),
            cipher,
            data: Default::default(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum HandshakeError<E: Error> {
    #[error(transparent)]
    ConnectionError(E),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error("Handshake failed")]
    HandshakeFailed,
    #[error("Encryption encountered an error")]
    AesError( aes_gcm::Error)
}

impl<E: Error> From<aes_gcm::Error> for HandshakeError<E> {
    fn from(e: aes_gcm::Error) -> Self {
        Self::AesError(e)
    }
}

impl<E: Error> HandshakeError<E> {
    pub fn new(error: E) -> Self {
        Self::ConnectionError(error)
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;
    use std::str::FromStr;
    use std::thread;
    use crate::handshake::{HandshakeListener, HandshakeStream};
    use crate::Stream;

    #[test]
    fn aes_encrypted() {
        let socket = SocketAddr::from_str("127.0.0.1:8888").unwrap();
        let mut connection = HandshakeListener::new(socket).unwrap();
        let thread = thread::spawn(move || {
            let mut server_stream = connection
                .accept::<String, String>()
                .expect("connection failed");
            server_stream.send("Hello, World".to_string()).unwrap();
        });

        let mut client_stream = HandshakeStream::<_, String>::bind(socket).unwrap();
        let packet = client_stream.read().unwrap();

        assert_eq!(packet.data(), "Hello, World");

        thread.join().unwrap();
    }

    #[test]
    fn can_disable_encryption() {
        let socket = SocketAddr::from_str("127.0.0.1:8888").unwrap();
        let mut connection = HandshakeListener::new(socket).unwrap();
        let thread = thread::spawn(move || {
            let mut server_stream = connection
                .accept::<String, String>()
                .expect("connection failed");
            server_stream.send("Hello, World".to_string()).unwrap();
            server_stream.send("Goodbye, World".to_string()).unwrap();
            server_stream.unwrap().send("Goodbye, World".to_string()).unwrap();
        });

        let mut client_stream = HandshakeStream::<_, String>::bind(socket).unwrap();
        let packet = client_stream.read().unwrap();

        assert_eq!(packet.data(), "Hello, World");
        let mut client_stream = client_stream.unwrap();
        client_stream.read().unwrap_err();
        let packet = client_stream.read().unwrap();

        assert_eq!(packet.data(), "Goodbye, World");

        thread.join().unwrap();
    }
}
