use thiserror::Error;

pub trait Compressor: Send + Sync {
    fn id(&self) -> u8;
    fn name(&self) -> &'static str;
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError>;
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError>;
}

#[derive(Debug, Error)]
#[error("Compression error: {message}")]
pub struct CompressionError {
    pub message: String,
}

impl CompressionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}
