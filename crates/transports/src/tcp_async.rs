use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use sora_protocol::async_transport::AsyncTransport;
use sora_protocol::transport::TransportError;

/// Async length-prefixed TCP transport using tokio.
pub struct AsyncTcpTransport {
    stream: TcpStream,
}

impl AsyncTcpTransport {
    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }

    pub fn stream(&self) -> &TcpStream {
        &self.stream
    }
}

impl AsyncTransport for AsyncTcpTransport {
    async fn send_bytes(&mut self, data: &[u8]) -> Result<(), TransportError> {
        let len = data.len() as u32;
        self.stream
            .write_all(&len.to_le_bytes())
            .await
            .map_err(|e| TransportError::from(e))?;
        self.stream
            .write_all(data)
            .await
            .map_err(|e| TransportError::from(e))?;
        self.stream
            .flush()
            .await
            .map_err(|e| TransportError::from(e))?;
        Ok(())
    }

    async fn recv_bytes(&mut self) -> Result<Vec<u8>, TransportError> {
        let mut len_buf = [0u8; 4];
        match self.stream.read_exact(&mut len_buf).await {
            Ok(_) => {}
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
        match self.stream.read_exact(&mut buf).await {
            Ok(_) => Ok(buf),
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                Err(TransportError::Closed)
            }
            Err(e) => Err(e.into()),
        }
    }
}
