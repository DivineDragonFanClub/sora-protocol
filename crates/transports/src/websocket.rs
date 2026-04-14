use std::io::{Read, Write};
use std::net::TcpStream;

use sora_protocol::transport::{Transport, TransportError};
use tungstenite::protocol::Role;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

/// WebSocket transport wrapping `tungstenite`.
///
/// WebSocket provides native message framing, so no length-prefix is needed.
/// All protocol messages are sent as binary WebSocket messages.
pub struct WebSocketTransport<S: Read + Write + Send> {
    ws: WebSocket<S>,
}

impl<S: Read + Write + Send> WebSocketTransport<S> {
    /// Wrap an existing tungstenite WebSocket.
    pub fn from_websocket(ws: WebSocket<S>) -> Self {
        Self { ws }
    }
}

impl WebSocketTransport<TcpStream> {
    /// Create a server-side WebSocket transport by accepting a WebSocket
    /// upgrade on an already-connected TCP stream.
    pub fn accept(stream: TcpStream) -> Result<Self, TransportError> {
        let ws = tungstenite::accept(stream)
            .map_err(|e| TransportError::Io(format!("WebSocket accept failed: {e}")))?;
        Ok(Self { ws })
    }

    /// Wrap a raw TCP stream as a WebSocket with the given role.
    /// Useful when the WebSocket handshake is handled externally.
    pub fn from_raw(stream: TcpStream, role: Role) -> Self {
        let ws = WebSocket::from_raw_socket(stream, role, None);
        Self { ws }
    }
}

impl WebSocketTransport<MaybeTlsStream<TcpStream>> {
    /// Create a client-side WebSocket transport by connecting to a URL.
    pub fn connect(url: &str) -> Result<Self, TransportError> {
        let (ws, _response) = tungstenite::connect(url)
            .map_err(|e| TransportError::Io(format!("WebSocket connect failed: {e}")))?;
        Ok(Self { ws })
    }
}

impl<S: Read + Write + Send> Transport for WebSocketTransport<S> {
    fn send_bytes(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.ws
            .send(Message::Binary(data.to_vec().into()))
            .map_err(ws_error_to_transport)
    }

    fn recv_bytes(&mut self) -> Result<Vec<u8>, TransportError> {
        loop {
            match self.ws.read() {
                Ok(Message::Binary(data)) => return Ok(data.into()),
                Ok(Message::Close(_)) => return Err(TransportError::Closed),
                // tungstenite auto-responds to pings
                Ok(Message::Ping(_) | Message::Pong(_) | Message::Text(_) | Message::Frame(_)) => {
                    continue
                }
                Err(e) => return Err(ws_error_to_transport(e)),
            }
        }
    }
}

fn ws_error_to_transport(err: tungstenite::Error) -> TransportError {
    match err {
        tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed => {
            TransportError::Closed
        }
        tungstenite::Error::Io(io_err) => io_err.into(),
        other => TransportError::Io(other.to_string()),
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
        let url = format!("ws://127.0.0.1:{}", addr.port());

        let server_thread = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            WebSocketTransport::accept(stream).unwrap()
        });

        let mut client = WebSocketTransport::connect(&url).unwrap();
        let mut server = server_thread.join().unwrap();

        client.send_bytes(b"hello").unwrap();
        let received = server.recv_bytes().unwrap();
        assert_eq!(received, b"hello");

        server.send_bytes(b"world").unwrap();
        let received = client.recv_bytes().unwrap();
        assert_eq!(received, b"world");
    }

    #[test]
    fn multiple_messages() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("ws://127.0.0.1:{}", addr.port());

        let server_thread = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            WebSocketTransport::accept(stream).unwrap()
        });

        let mut client = WebSocketTransport::connect(&url).unwrap();
        let mut server = server_thread.join().unwrap();

        for i in 0..10u8 {
            let msg = vec![i; (i as usize + 1) * 100];
            client.send_bytes(&msg).unwrap();
        }

        for i in 0..10u8 {
            let received = server.recv_bytes().unwrap();
            assert_eq!(received.len(), (i as usize + 1) * 100);
            assert!(received.iter().all(|&b| b == i));
        }
    }

    #[test]
    fn large_message() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("ws://127.0.0.1:{}", addr.port());

        let server_thread = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            WebSocketTransport::accept(stream).unwrap()
        });

        let mut client = WebSocketTransport::connect(&url).unwrap();
        let mut server = server_thread.join().unwrap();

        let data = vec![0xCD; 100_000];
        client.send_bytes(&data).unwrap();
        let received = server.recv_bytes().unwrap();
        assert_eq!(received, data);
    }
}
