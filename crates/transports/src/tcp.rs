use std::io::{Read, Write};
use std::net::TcpStream;

use sora_protocol::transport::{Transport, TransportError};

/// Each message: `[length: u32 LE][payload]`.
pub struct TcpTransport {
    stream: TcpStream,
}

impl TcpTransport {
    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }

    pub fn stream(&self) -> &TcpStream {
        &self.stream
    }

    pub fn stream_mut(&mut self) -> &mut TcpStream {
        &mut self.stream
    }
}

impl Transport for TcpTransport {
    fn send_bytes(&mut self, data: &[u8]) -> Result<(), TransportError> {
        let len = data.len() as u32;
        self.stream.write_all(&len.to_le_bytes())?;
        self.stream.write_all(data)?;
        self.stream.flush()?;
        Ok(())
    }

    fn recv_bytes(&mut self) -> Result<Vec<u8>, TransportError> {
        let mut len_buf = [0u8; 4];
        match self.stream.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(TransportError::Closed);
            }
            Err(e) => return Err(e.into()),
        }

        let len = u32::from_le_bytes(len_buf) as usize;

        const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;
        if len > MAX_MESSAGE_SIZE {
            return Err(TransportError::Io(format!(
                "Message too large: {len} bytes (max {MAX_MESSAGE_SIZE})"
            )));
        }

        let mut buf = vec![0u8; len];
        match self.stream.read_exact(&mut buf) {
            Ok(()) => Ok(buf),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                Err(TransportError::Closed)
            }
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn round_trip() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let client_stream = TcpStream::connect(addr).unwrap();
        let (server_stream, _) = listener.accept().unwrap();

        let mut client = TcpTransport::new(client_stream);
        let mut server = TcpTransport::new(server_stream);

        client.send_bytes(b"hello").unwrap();
        assert_eq!(server.recv_bytes().unwrap(), b"hello");

        server.send_bytes(b"world").unwrap();
        assert_eq!(client.recv_bytes().unwrap(), b"world");
    }

    #[test]
    fn empty_message() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let client_stream = TcpStream::connect(addr).unwrap();
        let (server_stream, _) = listener.accept().unwrap();

        let mut client = TcpTransport::new(client_stream);
        let mut server = TcpTransport::new(server_stream);

        client.send_bytes(b"").unwrap();
        assert_eq!(server.recv_bytes().unwrap(), b"");
    }

    #[test]
    fn large_message() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let client_stream = TcpStream::connect(addr).unwrap();
        let (server_stream, _) = listener.accept().unwrap();

        let mut client = TcpTransport::new(client_stream);
        let mut server = TcpTransport::new(server_stream);

        let data = vec![0xAB; 100_000];
        client.send_bytes(&data).unwrap();
        assert_eq!(server.recv_bytes().unwrap(), data);
    }

    #[test]
    fn multiple_messages() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let client_stream = TcpStream::connect(addr).unwrap();
        let (server_stream, _) = listener.accept().unwrap();

        let mut client = TcpTransport::new(client_stream);
        let mut server = TcpTransport::new(server_stream);

        for i in 0..10u8 {
            client.send_bytes(&vec![i; (i as usize + 1) * 100]).unwrap();
        }

        for i in 0..10u8 {
            let received = server.recv_bytes().unwrap();
            assert_eq!(received.len(), (i as usize + 1) * 100);
            assert!(received.iter().all(|&b| b == i));
        }
    }

    #[test]
    fn closed_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let client_stream = TcpStream::connect(addr).unwrap();
        let (server_stream, _) = listener.accept().unwrap();

        let mut server = TcpTransport::new(server_stream);
        drop(client_stream);

        assert!(matches!(server.recv_bytes(), Err(TransportError::Closed)));
    }
}
