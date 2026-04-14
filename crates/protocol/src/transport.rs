use thiserror::Error;

pub trait Transport: Send {
    fn send_bytes(&mut self, data: &[u8]) -> Result<(), TransportError>;
    fn recv_bytes(&mut self) -> Result<Vec<u8>, TransportError>;
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("Connection closed")]
    Closed,
    #[error("Transport I/O error: {0}")]
    Io(String),
    #[error("Transport operation timed out")]
    TimedOut,
}

impl From<std::io::Error> for TransportError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => Self::TimedOut,
            std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted | std::io::ErrorKind::BrokenPipe => Self::Closed,
            _ => Self::Io(err.to_string()),
        }
    }
}
